//! Lớp mỏng trên tract: nạp mô hình ONNX từ byte ĐÃ KIỂM toàn vẹn, khai báo hình dạng vào, chạy.
//!
//! - Không đọc tệp phụ (external data) qua đường dẫn: mọi trọng số phải nằm trong byte đã kiểm sha256.
//! - Vá đồ thị trước khi dịch: nút `If` sinh từ `torch.squeeze(dim)` (nhánh then = `Squeeze`, nhánh else =
//!   `Identity`) được thay bằng `Squeeze` vì tract không nhận `If` có hai nhánh khác hạng. `Squeeze` của tract báo lỗi
//!   nếu chiều đó khác 1 ⇒ sai thì dừng, không cho kết quả âm thầm sai.

use std::io::Cursor;
use std::sync::{Arc, Mutex};

use tract_onnx::pb;
use tract_onnx::prelude::*;

use crate::error::{MlError, Result};

/// Một chiều của đầu vào: cố định hoặc ký hiệu (thay đổi theo ảnh).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dim {
    /// Kích thước cố định.
    Fixed(usize),
    /// Kích thước biến thiên, tên ký hiệu.
    Symbol(&'static str),
    /// Kích thước biến thiên là bội số của một số nguyên (`n · k`), giúp tract rút gọn phép chia khi hạ mẫu.
    Multiple(&'static str, i64),
}

/// Số kế hoạch theo kích thước giữ trong bộ đệm mặc định (chế độ hình dạng cụ thể).
const SHAPE_CACHE: usize = 6;

/// Phiên suy luận đã tối ưu, dùng lại được cho nhiều ảnh.
///
/// Thử tối ưu một lần với hình dạng ký hiệu. Nếu tract không suy luận được (một số mô hình hạ mẫu có phép chia ký
/// hiệu không rút gọn được), chuyển sang chế độ biên dịch theo từng kích thước cụ thể và giữ [`SHAPE_CACHE`] kế hoạch
/// gần nhất - bên gọi nên làm tròn kích thước ảnh để số kích thước khác nhau nhỏ.
#[derive(Debug)]
pub struct Session {
    name: String,
    symbolic: Option<Arc<TypedRunnableModel>>,
    template: Option<InferenceModel>,
    cache: Mutex<Vec<(Vec<usize>, Arc<TypedRunnableModel>)>>,
    capacity: usize,
}

impl Session {
    /// Dịch và tối ưu mô hình từ byte ONNX (đã kiểm sha256 bởi [`crate::manifest::ModelStore`]).
    ///
    /// # Errors
    /// Protobuf hỏng, toán tử không hỗ trợ, có external data, hoặc hình dạng không khớp.
    pub fn from_bytes(name: &str, bytes: &[u8], input: &[Dim]) -> Result<Self> {
        Self::from_bytes_with_output(name, bytes, input, None)
    }

    /// Như [`Self::from_bytes`] nhưng cắt đồ thị tại tensor trung gian `output` (bỏ phần đuôi tự làm bằng Rust).
    ///
    /// # Errors
    /// Như [`Self::from_bytes`], hoặc không có tensor tên `output`.
    pub fn from_bytes_with_output(
        name: &str,
        bytes: &[u8],
        input: &[Dim],
        output: Option<&str>,
    ) -> Result<Self> {
        let mut template = parse(name, bytes)?;
        if let Some(o) = output {
            template.select_outputs_by_name([o])?;
        }
        let mut model = template.clone();
        let dims: TVec<TDim> = input
            .iter()
            .map(|d| match d {
                Dim::Fixed(n) => n.to_dim(),
                Dim::Symbol(s) => model.symbols.sym(s).into(),
                Dim::Multiple(s, k) => TDim::from(model.symbols.sym(s)) * *k,
            })
            .collect();
        model.set_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), dims))?;
        let has_symbols = input.iter().any(|d| !matches!(d, Dim::Fixed(_)));
        match model.into_optimized().and_then(IntoRunnable::into_runnable) {
            Ok(plan) => Ok(Self {
                name: name.to_owned(),
                symbolic: Some(plan),
                template: None,
                cache: Mutex::new(Vec::new()),
                capacity: SHAPE_CACHE,
            }),
            Err(_) if has_symbols => Ok(Self::per_shape_from(name, template, SHAPE_CACHE)),
            Err(e) => Err(MlError::Inference(format!("{name}: {e:#}"))),
        }
    }

    /// Luôn biên dịch theo kích thước cụ thể (nhanh hơn 2-3 lần khi chạy so với hình dạng ký hiệu với mô hình có
    /// LSTM/attention), giữ tối đa `capacity` kế hoạch. Bên gọi `MUST` làm tròn kích thước vào thành ít "xô".
    ///
    /// # Errors
    /// Như [`Self::from_bytes`].
    pub fn per_shape(name: &str, bytes: &[u8], capacity: usize) -> Result<Self> {
        Ok(Self::per_shape_from(name, parse(name, bytes)?, capacity.max(1)))
    }

    fn per_shape_from(name: &str, template: InferenceModel, capacity: usize) -> Self {
        Self {
            name: name.to_owned(),
            symbolic: None,
            template: Some(template),
            cache: Mutex::new(Vec::new()),
            capacity,
        }
    }

    /// Chế độ đang dùng có phải hình dạng ký hiệu (một kế hoạch cho mọi kích thước) không.
    #[must_use]
    pub fn is_symbolic(&self) -> bool {
        self.symbolic.is_some()
    }

    fn plan_for(&self, shape: &[usize]) -> Result<Arc<TypedRunnableModel>> {
        if let Some(p) = &self.symbolic {
            return Ok(Arc::clone(p));
        }
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| MlError::Inference(format!("{}: bộ đệm hỏng", self.name)))?;
        if let Some(pos) = cache.iter().position(|(s, _)| s == shape) {
            let hit = cache.remove(pos);
            let plan = Arc::clone(&hit.1);
            cache.push(hit);
            return Ok(plan);
        }
        let mut model = self
            .template
            .clone()
            .ok_or_else(|| MlError::Inference(format!("{}: thiếu mẫu", self.name)))?;
        model.set_input_fact(0, f32::fact(shape).into())?;
        let plan = model.into_optimized()?.into_runnable()?;
        if cache.len() >= self.capacity {
            cache.remove(0);
        }
        cache.push((shape.to_vec(), Arc::clone(&plan)));
        Ok(plan)
    }

    /// Chạy với một tensor f32 vào, trả về các tensor ra.
    ///
    /// # Errors
    /// Hình dạng vào không khớp hoặc lỗi tính toán.
    pub fn run(&self, input: Tensor) -> Result<TVec<TValue>> {
        let plan = self.plan_for(input.shape())?;
        plan.run(tvec![input.into()])
            .map_err(|e| MlError::Inference(format!("{}: {e:#}", self.name)))
    }
}

/// Đọc protobuf, kiểm external data, vá `If`, dịch sang mô hình suy luận (chưa gắn hình dạng vào).
fn parse(name: &str, bytes: &[u8]) -> Result<InferenceModel> {
    let onnx = tract_onnx::onnx()
        .with_ignore_output_shapes(true)
        .with_ignore_value_info(true);
    let mut proto = onnx.proto_model_for_read(&mut Cursor::new(bytes))?;
    let graph = proto
        .graph
        .as_mut()
        .ok_or_else(|| MlError::Inference(format!("{name}: không có đồ thị")))?;
    reject_external_data(graph).map_err(|e| MlError::Inference(format!("{name}: {e}")))?;
    inline_squeeze_ifs(graph);
    strip_pad_constant_for_non_constant_mode(graph);
    Ok(onnx.model_for_proto_model(&proto)?)
}

/// Từ chối mô hình trỏ tới trọng số ngoài (external data): chúng nằm ngoài vùng đã kiểm sha256.
fn reject_external_data(graph: &pb::GraphProto) -> std::result::Result<(), String> {
    // data_location 1 = EXTERNAL.
    if graph
        .initializer
        .iter()
        .any(|t| t.data_location == Some(1) || !t.external_data.is_empty())
    {
        return Err("mô hình dùng external data - không được hỗ trợ".into());
    }
    Ok(())
}

/// Thay mẫu `If(cond){Squeeze(x, axes)} else {Identity(x)}` bằng `Squeeze(x, axes)`.
fn inline_squeeze_ifs(graph: &mut pb::GraphProto) {
    let mut extra = Vec::new();
    for node in &mut graph.node {
        if node.op_type != "If" || node.output.len() != 1 {
            continue;
        }
        let branch = |n: &str| {
            node.attribute
                .iter()
                .find(|a| a.name == n)
                .and_then(|a| a.g.clone())
        };
        let (Some(then_g), Some(else_g)) = (branch("then_branch"), branch("else_branch")) else {
            continue;
        };
        let is_identity = else_g.node.len() == 1 && else_g.node[0].op_type == "Identity";
        let squeeze = then_g.node.iter().find(|n| n.op_type == "Squeeze");
        let consts: Vec<&pb::NodeProto> = then_g.node.iter().filter(|n| n.op_type == "Constant").collect();
        let Some(squeeze) = squeeze else { continue };
        if !is_identity
            || then_g.node.len() != consts.len() + 1
            || squeeze.input.first() != else_g.node[0].input.first()
        {
            continue;
        }
        // Đưa các Constant (trục squeeze) ra đồ thị ngoài, đổi Squeeze ghi thẳng vào đầu ra của If.
        extra.extend(consts.into_iter().cloned());
        let mut sq = squeeze.clone();
        sq.output.clone_from(&node.output);
        sq.name = format!("{}_inlined", node.name);
        *node = sq;
    }
    // Constant phải đứng trước nút dùng nó: chèn lên đầu (ONNX đã sắp xếp tô-pô, Constant không có đầu vào).
    graph.node.splice(0..0, extra);
}

/// tract 0.23 dùng chế độ hằng cho `Pad` khi có đầu vào `constant_value`, kể cả khi `mode` là `reflect`/`edge`
/// (UVDoc: lưới sai hoàn toàn, đối chiếu onnxruntime 18/11/2026). Theo chuẩn ONNX, `constant_value` chỉ có nghĩa với
/// `mode=constant` ⇒ bỏ đầu vào đó ở các chế độ khác là giữ nguyên ngữ nghĩa.
fn strip_pad_constant_for_non_constant_mode(graph: &mut pb::GraphProto) {
    for n in &mut graph.node {
        let non_constant = n
            .attribute
            .iter()
            .any(|a| a.name == "mode" && (a.s == b"reflect" || a.s == b"edge"));
        if n.op_type == "Pad" && non_constant && n.input.len() >= 3 {
            n.input[2].clear(); // đầu vào tùy chọn bỏ trống
            while n.input.last().is_some_and(String::is_empty) {
                n.input.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(op: &str, inputs: &[&str], outputs: &[&str]) -> pb::NodeProto {
        pb::NodeProto {
            op_type: op.into(),
            input: inputs.iter().map(|s| (*s).to_owned()).collect(),
            output: outputs.iter().map(|s| (*s).to_owned()).collect(),
            name: format!("{op}_{}", outputs.first().copied().unwrap_or_default()),
            ..Default::default()
        }
    }

    fn graph_attr(name: &str, nodes: Vec<pb::NodeProto>) -> pb::AttributeProto {
        pb::AttributeProto {
            name: name.into(),
            g: Some(pb::GraphProto {
                node: nodes,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn thay_if_squeeze_bang_squeeze() {
        let mut ifn = node("If", &["cond"], &["y"]);
        ifn.attribute = vec![
            graph_attr(
                "then_branch",
                vec![
                    node("Constant", &[], &["axes"]),
                    node("Squeeze", &["x", "axes"], &["s"]),
                ],
            ),
            graph_attr("else_branch", vec![node("Identity", &["x"], &["i"])]),
        ];
        let mut g = pb::GraphProto {
            node: vec![node("Relu", &["a"], &["x"]), ifn],
            ..Default::default()
        };
        inline_squeeze_ifs(&mut g);
        let ops: Vec<&str> = g.node.iter().map(|n| n.op_type.as_str()).collect();
        assert_eq!(ops, ["Constant", "Relu", "Squeeze"]);
        assert_eq!(g.node[2].output, ["y"]);
        assert_eq!(g.node[2].input, ["x", "axes"]);
    }

    #[test]
    fn giu_nguyen_if_khac_mau() {
        let mut ifn = node("If", &["cond"], &["y"]);
        ifn.attribute = vec![
            graph_attr("then_branch", vec![node("Relu", &["x"], &["s"])]),
            graph_attr("else_branch", vec![node("Identity", &["x"], &["i"])]),
        ];
        let mut g = pb::GraphProto {
            node: vec![ifn],
            ..Default::default()
        };
        inline_squeeze_ifs(&mut g);
        assert_eq!(g.node[0].op_type, "If");
    }

    #[test]
    fn bo_constant_value_cua_pad_reflect() {
        let mut pad = node("Pad", &["x", "pads", "cv"], &["p"]);
        pad.attribute = vec![pb::AttributeProto {
            name: "mode".into(),
            s: b"reflect".to_vec(),
            ..Default::default()
        }];
        let mut cpad = node("Pad", &["x", "pads", "cv"], &["q"]);
        cpad.attribute = vec![pb::AttributeProto {
            name: "mode".into(),
            s: b"constant".to_vec(),
            ..Default::default()
        }];
        let mut axes = node("Pad", &["x", "pads", "cv", "axes"], &["r"]);
        axes.attribute = pad.attribute.clone();
        let mut g = pb::GraphProto {
            node: vec![pad, cpad, axes],
            ..Default::default()
        };
        strip_pad_constant_for_non_constant_mode(&mut g);
        assert_eq!(g.node[0].input, ["x", "pads"]);
        assert_eq!(g.node[1].input, ["x", "pads", "cv"]);
        assert_eq!(g.node[2].input, ["x", "pads", "", "axes"]);
    }

    #[test]
    fn tu_choi_byte_rac() {
        assert!(Session::from_bytes("rac", b"khong phai onnx", &[Dim::Fixed(1)]).is_err());
    }

    #[test]
    fn tu_choi_external_data() {
        let g = pb::GraphProto {
            initializer: vec![pb::TensorProto {
                data_location: Some(1),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(reject_external_data(&g).is_err());
        assert!(reject_external_data(&pb::GraphProto::default()).is_ok());
    }
}
