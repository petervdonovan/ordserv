use std::{
  collections::HashMap,
  hash::{Hash, Hasher},
  sync::{Arc, RwLock, RwLockReadGuard},
};

use serde::{Deserialize, Serialize};
use streaming_transpositions::{CurRank, OgRank, OgRank2CurRank};

pub(crate) const OUTPUT_VECTOR_CHUNK_SIZE: usize = 32;

use crate::{
  state::{TestMetadata, TracePointId},
  testing::{TraceHash, TraceHasher},
  TraceRecord,
};
impl TestMetadata {
  pub fn og_ov_length_rounded_up(&self) -> usize {
    ((self.out_ovkey.n_tracepoints + OUTPUT_VECTOR_CHUNK_SIZE - 1) / OUTPUT_VECTOR_CHUNK_SIZE)
      * OUTPUT_VECTOR_CHUNK_SIZE
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct OutputVector {
  data: OutputVectorNodeIdx,
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
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct OutputVectorNodeIdx(u64);
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct OutputVectorChunk {
  rel_ranks: [i32; OUTPUT_VECTOR_CHUNK_SIZE], // relative to og rank
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
struct OutputVectorNodePair {
  left: OutputVectorNodeIdx,
  right: Option<OutputVectorNodeIdx>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OutputVectorKey {
  pub records: Vec<TraceRecord>,
  pub map: HashMap<TracePointId, Vec<OgRank>>,
  pub n_tracepoints: usize,
}
#[derive(Debug, Default)]
pub struct OvrReg {
  idx2node: Vec<OutputVectorNode>,
  idx2node_saved_up_to: OutputVectorNodeIdx,
  node2idx: HashMap<OutputVectorNode, OutputVectorNodeIdx>, // FIXME: 128-bit hash
}
impl Serialize for OvrReg {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    (OvrDelta {
      idx2node: self.idx2node[self.idx2node_saved_up_to.0 as usize..]
        .iter()
        .enumerate()
        .map(|(idx, node)| (idx + (self.idx2node_saved_up_to.0 as usize), *node))
        .collect(),
    })
    .serialize(serializer)
  }
}
#[derive(Deserialize, Serialize)]
pub struct OvrDelta {
  idx2node: Vec<(usize, OutputVectorNode)>,
}
impl OvrReg {
  pub fn update_saved_up_to_for_saving_deltas(&mut self) {
    self.idx2node_saved_up_to = OutputVectorNodeIdx(self.idx2node.len() as u64);
  }
  pub fn rebuild(deltas: impl Iterator<Item = OvrDelta>) -> Self {
    let mut ret = Self::default();
    for (deltaidx, delta) in deltas.enumerate() {
      for (deltasubidx, (ogidx, ov)) in delta.idx2node.iter().enumerate() {
        if *ogidx != ret.idx2node.len() {
          panic!("indexing mismatch; check if deltas were provided in the right order. Got index {} but expected {} after loading {} deltas and {} subthings.", ogidx, ret.idx2node.len(), deltaidx, deltasubidx);
        }
        ret.node2idx.insert(
          compute_hash(*ov),
          OutputVectorNodeIdx(ret.idx2node.len() as u64),
        );
        ret.idx2node.push(*ov);
      }
    }
    ret.update_saved_up_to_for_saving_deltas();
    ret
  }
}
pub type OutputVectorRegistry = Arc<RwLock<OvrReg>>;

impl OutputVectorKey {
  pub fn new(tpis: Vec<TraceRecord>, round_up_to_zero_mod: usize) -> Self {
    let mut ret = HashMap::new();
    let mut idx = 0;
    for tpi in tpis.iter().map(TracePointId::new) {
      ret.entry(tpi).or_insert(vec![]).push(OgRank(idx));
      idx += 1;
    }
    Self {
      records: tpis,
      map: ret,
      n_tracepoints: (idx as usize - 1) / round_up_to_zero_mod * round_up_to_zero_mod
        + round_up_to_zero_mod,
    }
  }

  pub fn vectorfy(
    &self,
    records: impl Iterator<Item = TraceRecord>,
  ) -> (OgRank2CurRank, TraceHash, VectorfyStatus) {
    let mut ov = vec![CurRank(self.n_tracepoints as u32); self.n_tracepoints];
    let mut th = TraceHasher::default();
    let mut status = VectorfyStatus::Ok;
    let mut subidxs = HashMap::new();
    for (rank, tr) in records.enumerate() {
      let tpi = TracePointId::new(&tr);
      if let Some(idxs) = self.map.get(&tpi) {
        subidxs.entry(tpi).or_insert(0);
        if let Some(idx) = idxs.get(subidxs[&tpi]) {
          ov[idx.0 as usize] = CurRank(rank as u32);
          subidxs.entry(tpi).and_modify(|it| *it += 1);
        } else {
          status = VectorfyStatus::ExtraTracePointId;
        }
      } else {
        status = VectorfyStatus::MissingTracePointId;
      }
      th.update(&tr);
    }
    (OgRank2CurRank(ov), th.finish(), status)
  }
  #[allow(clippy::len_without_is_empty)]
  pub fn len(&self) -> usize {
    self.n_tracepoints
  }
}
#[derive(Debug, Serialize, Deserialize)]
pub enum VectorfyStatus {
  Ok,
  MissingTracePointId,
  ExtraTracePointId,
}

fn compute_hash(ovn: OutputVectorNode) -> OutputVectorNode {
  ovn
}

impl OutputVector {
  pub fn new(ov: OgRank2CurRank, ovr: OutputVectorRegistry) -> Self {
    let mut ovrmut = ovr.write().unwrap();
    let data = Self::new_rec(&ov.0, &mut ovrmut, 0, ov.0.len() as i32);
    Self {
      data,
      len: ov.0.len(),
    }
  }
  fn new_rec(ov: &[CurRank], ovr: &mut OvrReg, start: usize, default: i32) -> OutputVectorNodeIdx {
    // Remark: it is impressive that after copilot generated this function, only small edits were
    // required.
    if ov.len() <= OUTPUT_VECTOR_CHUNK_SIZE {
      let mut ranks = [default; OUTPUT_VECTOR_CHUNK_SIZE];
      for (i, rank) in ov.iter().enumerate() {
        ranks[i] = (rank.0 as i32) - (start as i32);
      }
      let chunk = OutputVectorNode::Leaf(OutputVectorChunk { rel_ranks: ranks });
      if let Some(id) = ovr.node2idx.get(&compute_hash(chunk)) {
        *id
      } else {
        let id = OutputVectorNodeIdx(ovr.idx2node.len() as u64);
        ovr.idx2node.push(chunk);
        ovr.node2idx.insert(compute_hash(chunk), id);
        id
      }
    } else {
      let mid = ov.len().next_power_of_two() / 2;
      let left = Self::new_rec(&ov[..mid], ovr, start, default);
      let right = Self::new_rec(&ov[mid..], ovr, start + mid, default);
      let pair = OutputVectorNode::Node(OutputVectorNodePair {
        left,
        right: Some(right),
      });
      if let Some(id) = ovr.node2idx.get(&compute_hash(pair)) {
        *id
      } else {
        let id = OutputVectorNodeIdx(ovr.idx2node.len() as u64);
        ovr.idx2node.push(pair);
        ovr.node2idx.insert(compute_hash(pair), id);
        id
      }
    }
  }
  pub fn unpack(&self, ovr: &OutputVectorRegistry) -> Vec<CurRank> {
    let rounded_up_len = (self.len - 1) / OUTPUT_VECTOR_CHUNK_SIZE * OUTPUT_VECTOR_CHUNK_SIZE
      + OUTPUT_VECTOR_CHUNK_SIZE;
    let default = CurRank(self.len as u32);
    let mut ret = vec![default; rounded_up_len];
    Self::unpack_rec(self.data, &ovr.read().unwrap(), &mut ret, 0, default);
    ret
  }
  fn unpack_rec(
    ovnid: OutputVectorNodeIdx,
    ovrdata: &RwLockReadGuard<'_, OvrReg>,
    seqnum2hookinvoc: &mut [CurRank],
    start: u32,
    default: CurRank,
  ) {
    if seqnum2hookinvoc.len() / OUTPUT_VECTOR_CHUNK_SIZE * OUTPUT_VECTOR_CHUNK_SIZE
      != seqnum2hookinvoc.len()
    {
      eprintln!("seqnum2hookinvoc.len() = {}", seqnum2hookinvoc.len());
      panic!("seqnum2hookinvoc.len() must be a multiple of OUTPUT_VECTOR_CHUNK_SIZE");
    }
    match ovrdata.idx2node[ovnid.0 as usize] {
      OutputVectorNode::Leaf(chunk) => {
        if seqnum2hookinvoc.len() != OUTPUT_VECTOR_CHUNK_SIZE {
          panic!("seqnum2hookinvoc.len() must be OUTPUT_VECTOR_CHUNK_SIZE when unpacking a leaf, but it is {}", seqnum2hookinvoc.len());
        }
        for (i, rank) in chunk.rel_ranks.iter().enumerate() {
          if *rank == default.0 as i32 {
            continue;
          }
          seqnum2hookinvoc[i] = CurRank((*rank + start as i32) as u32);
        }
      }
      OutputVectorNode::Node(pair) => {
        let mid = seqnum2hookinvoc.len().next_power_of_two() / 2;
        Self::unpack_rec(
          pair.left,
          ovrdata,
          &mut seqnum2hookinvoc[0..mid],
          start,
          default,
        );
        if let Some(right) = pair.right {
          Self::unpack_rec(
            right,
            ovrdata,
            &mut seqnum2hookinvoc[mid..],
            start + mid as u32,
            default,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {

  use rand::seq::SliceRandom;

  use super::*;

  #[test]
  fn test_output_vector() {
    let length = 723;
    let rounded_up =
      (length - 1) / OUTPUT_VECTOR_CHUNK_SIZE * OUTPUT_VECTOR_CHUNK_SIZE + OUTPUT_VECTOR_CHUNK_SIZE;
    let ovr = Arc::new(RwLock::new(OvrReg::default()));
    let og_trace = (0..length).map(|_| TraceRecord::mock()).collect::<Vec<_>>();
    let mut new_trace = og_trace.clone();
    for _ in 0..23 {
      let start = rand::random::<usize>() % length;
      let end = rand::random::<usize>() % 14;
      new_trace[start..end.max(length)].shuffle(&mut rand::thread_rng());
    }
    let ovk = OutputVectorKey::new(og_trace, 1);
    let (ov, _, _) = ovk.vectorfy(new_trace.clone().into_iter());
    let ov = OutputVector::new(ov, Arc::clone(&ovr));
    let ov = ov.unpack(&ovr);
    let new_trace_og_ranks = new_trace
      .iter()
      .map(|id| ovk.map[&TracePointId::new(id)][0].0)
      .enumerate()
      .map(|(i, rank)| (rank, i as u32))
      .collect::<HashMap<_, _>>();
    assert_eq!(
      ov,
      (0..rounded_up)
        .map(|it| *new_trace_og_ranks
          .get(&(it as u32))
          .unwrap_or(&(length as u32)))
        .map(CurRank)
        .collect::<Vec<_>>()
    );
  }
}
