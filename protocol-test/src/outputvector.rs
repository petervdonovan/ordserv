use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

const OUTPUT_VECTOR_CHUNK_SIZE: usize = 32;

use crate::{
  state::TracePointId,
  testing::{SuccessfulRun, TraceHasher},
  TraceRecord,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputVector {
  data: OutputVectorNodeId,
  len: usize,
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum OutputVectorNode {
  Leaf(OutputVectorChunk),
  Node(OutputVectorNodePair),
}
impl Hash for OutputVectorNode {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      OutputVectorNode::Leaf(chunk) => {
        chunk.rel_ranks.hash(state);
      }
      OutputVectorNode::Node(pair) => {
        pair.left.hash(state);
        pair.right.hash(state);
      }
    }
  }
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct OutputVectorNodeId(u64);
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct OutputVectorChunk {
  rel_ranks: [i32; OUTPUT_VECTOR_CHUNK_SIZE], // relative to og rank
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct OutputVectorNodePair {
  left: OutputVectorNodeId,
  right: Option<OutputVectorNodeId>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OutputVectorKey {
  pub map: HashMap<TracePointId, Vec<usize>>,
  pub n_tracepoints: usize,
  registry: OutputVectorRegistry,
}
#[derive(Debug, Serialize, Deserialize)]
struct OvrData {
  id2node: Vec<OutputVectorNode>,
  node2id: HashMap<u64, OutputVectorNodeId>, // FIXME: 128-bit hash
}
type OutputVectorRegistry = Arc<Mutex<OvrData>>;

impl OvrData {
  fn empty() -> Self {
    Self {
      id2node: vec![],
      node2id: HashMap::new(),
    }
  }
}

impl OutputVectorKey {
  pub fn new(tpis: impl Iterator<Item = TracePointId>) -> Self {
    let mut ret = HashMap::new();
    let mut idx = 0;
    for tpi in tpis {
      ret.entry(tpi).or_insert(vec![]).push(idx);
      idx += 1;
    }
    Self {
      map: ret,
      n_tracepoints: idx + 1,
      registry: Arc::new(Mutex::new(OvrData::empty())),
    }
  }

  pub fn vectorfy(&self, records: impl Iterator<Item = TraceRecord>) -> SuccessfulRun {
    let mut ov = vec![0; self.n_tracepoints];
    let mut th = TraceHasher::default();
    let mut status = VectorfyStatus::Ok;
    let mut subidxs = HashMap::new();
    for (rank, tr) in records.enumerate() {
      let tpi = TracePointId::new(&tr);
      if let Some(idxs) = self.map.get(&tpi) {
        subidxs.entry(tpi).or_insert(0);
        if let Some(idx) = idxs.get(subidxs[&tpi]) {
          ov[*idx] = rank as u32;
        } else {
          status = VectorfyStatus::ExtraTracePointId;
        }
      } else {
        status = VectorfyStatus::MissingTracePointId;
      }
      th.update(&tr);
    }
    (
      OutputVector::new(ov, self.registry.clone()),
      th.finish(),
      status,
    )
  }
}
#[derive(Debug, Serialize, Deserialize)]
pub enum VectorfyStatus {
  Ok,
  MissingTracePointId,
  ExtraTracePointId,
}

fn compute_hash(ovn: OutputVectorNode) -> u64 {
  let mut hasher = DefaultHasher::default();
  ovn.hash(&mut hasher);
  hasher.finish()
}

impl OutputVector {
  fn new(ov: Vec<u32>, ovr: OutputVectorRegistry) -> Self {
    let ovr = ovr.clone();
    let mut ovrmut = ovr.lock().unwrap();
    let data = Self::new_rec(&ov, &mut ovrmut, 0);
    Self {
      data,
      len: ov.len(),
    }
  }
  fn new_rec(ov: &[u32], ovr: &mut OvrData, start: usize) -> OutputVectorNodeId {
    // Remark: it is impressive that after copilot generated this function, only small edits were
    // required.
    if ov.len() <= OUTPUT_VECTOR_CHUNK_SIZE {
      let mut ranks = [0; OUTPUT_VECTOR_CHUNK_SIZE];
      for (i, rank) in ov.iter().enumerate() {
        ranks[i] = (*rank as i32) - (start as i32);
      }
      let chunk = OutputVectorNode::Leaf(OutputVectorChunk { rel_ranks: ranks });
      if let Some(id) = ovr.node2id.get(&compute_hash(chunk)) {
        *id
      } else {
        let id = OutputVectorNodeId(ovr.id2node.len() as u64);
        ovr.id2node.push(chunk);
        ovr.node2id.insert(compute_hash(chunk), id);
        id
      }
    } else {
      let mid = (ov.len() / 2).next_power_of_two();
      let left = Self::new_rec(&ov[..mid], ovr, start);
      let right = Self::new_rec(&ov[mid..], ovr, mid);
      let pair = OutputVectorNode::Node(OutputVectorNodePair {
        left,
        right: Some(right),
      });
      if let Some(id) = ovr.node2id.get(&compute_hash(pair)) {
        *id
      } else {
        let id = OutputVectorNodeId(ovr.id2node.len() as u64);
        ovr.id2node.push(pair);
        ovr.node2id.insert(compute_hash(pair), id);
        id
      }
    }
  }
}
