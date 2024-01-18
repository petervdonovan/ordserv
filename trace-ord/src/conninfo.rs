use std::{collections::BTreeMap, fmt::Display, ops::Add, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Delay(u64, u64);

pub const NO_DELAY: Delay = Delay(0, 0);
pub const STARTUP: Tag = Tag(0, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Tag(pub i64, pub u64);

impl Add<Delay> for Tag {
    type Output = Self;

    fn add(self, rhs: Delay) -> Self::Output {
        let mut tag = self;
        if tag.0 == i64::MAX || rhs.0 == u64::MAX {
            tag.0 = i64::MAX;
        } else {
            tag.0 += rhs.0 as i64;
        }
        if tag.1 == u64::MAX || rhs.1 == u64::MAX {
            tag.1 = u64::MAX;
        } else {
            tag.1 += rhs.1;
        }
        tag
    }
}

impl Tag {
    pub fn strict_plus(self, rhs: Delay) -> Self {
        let mut tag = self;
        tag.0 += rhs.0 as i64;
        tag.1 += rhs.1;
        if rhs.0 != 0 {
            tag.0 -= 1;
            tag.1 = if tag.1 == 0 { u64::MAX } else { tag.1 - 1 };
        }
        tag
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl FromStr for Tag {
    type Err = String;
    /// inverse of Display. Format: (i64, i64)
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip the parentheses
        let s = &s[1..s.len() - 1];
        let mut tag = s.split(", ");
        let time = 1000
            * tag
                .next()
                .ok_or("No time")?
                .parse::<i64>()
                .map_err(|e| format!("Invalid time: {}", e))?;
        let microstep = 1000
            * tag
                .next()
                .ok_or("No count")?
                .parse::<i64>()
                .map_err(|e| format!("Invalid count: {}", e))?;
        Ok(Self(time, get_nonnegative_microstep(microstep)))
    }
}

pub fn get_nonnegative_microstep(microstep: i64) -> u64 {
    if microstep == -1 {
        u64::MAX
    } else if microstep < 0 {
        panic!("Negative microstep");
    } else {
        microstep as u64
    }
}

impl Display for Delay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 && self.1 == 0 {
            write!(f, "no_delay")
        } else if self.0 == 0 && self.1 == 1 {
            write!(f, "(0, 0)")
        } else if self.0 > 0 {
            write!(f, "{}", self.0)
        } else {
            unreachable!()
        }
    }
}

impl From<Delay> for i64 {
    fn from(delay: Delay) -> Self {
        if delay.0 == 0 && delay.1 == 0 {
            -9223372036854775808
        } else if delay.0 == 0 && delay.1 == 1 {
            0
        } else if delay.0 > 0 {
            delay.0 as i64
        } else {
            panic!("Negative interval");
        }
    }
}

impl From<i64> for Delay {
    fn from(interval: i64) -> Self {
        if interval == -9223372036854775808 {
            Self(0, 0)
        } else if interval == 0 {
            Self(0, 1)
        } else if interval > 0 {
            Self(interval as u64, 0)
        } else {
            panic!("Negative interval");
        }
    }
}

impl FromStr for Delay {
    type Err = String;

    /// Format:
    ///   i64
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut delay = s.split_whitespace();
        let interval = delay
            .next()
            .ok_or("No interval")?
            .parse::<i64>()
            .map_err(|e| format!("Invalid interval: {}", e))?;
        Ok(Self::from(interval))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FedId(pub i32);
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnInfo {
    stdp2d: SrcDestPair2Delay,
    fed2uds: Vec<Fed2UpstreamDelays>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SrcDestPair2Delay(BTreeMap<(FedId, FedId), Delay>, usize); // use BTreeMap instead of HashMap to make the order deterministic when serializing
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Fed2UpstreamDelays(FedId, Vec<Delay>);

impl FromStr for SrcDestPair2Delay {
    type Err = String;

    /// Format:
    ///   number_of_scheduling_nodes
    ///   (enclave_id num_upstream (upstream_federate_id upstream_delay)*\n)*
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut conn_info = BTreeMap::new();
        let mut lines = s.lines();
        let num_nodes = lines
            .next()
            .ok_or("No number of scheduling nodes")?
            .parse::<usize>()
            .map_err(|e| format!("Invalid number of scheduling nodes: {}", e))?;
        for _ in 0..num_nodes {
            let mut line = lines.next().ok_or("No scheduling node")?.split_whitespace();
            let enclave_id = line
                .next()
                .ok_or("No enclave id")?
                .parse::<i32>()
                .map_err(|e| format!("Invalid enclave id: {}", e))?;
            let num_upstream = line
                .next()
                .ok_or("No number of upstream federates")?
                .parse::<usize>()
                .map_err(|e| format!("Invalid number of upstream federates: {}", e))?;
            for _ in 0..num_upstream {
                let upstream_fed_id = line
                    .next()
                    .ok_or("No upstream federate id")?
                    .parse::<i32>()
                    .map_err(|e| format!("Invalid upstream federate id: {}", e))?;
                let upstream_delay = line
                    .next()
                    .ok_or("No upstream delay")?
                    .parse::<Delay>()
                    .map_err(|e| format!("Invalid upstream delay: {}", e))?;
                conn_info.insert((FedId(upstream_fed_id), FedId(enclave_id)), upstream_delay);
            }
        }
        Ok(Self(conn_info, num_nodes))
    }
}

impl FromStr for Fed2UpstreamDelays {
    type Err = String;

    /// Format:
    ///  federate_id n_delays delay0 delay1 delay2 ...
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut line = s.split_whitespace();
        let fed_id = line
            .next()
            .ok_or("No federate id")?
            .parse::<i32>()
            .map_err(|e| format!("Invalid federate id: {}", e))?;
        let n_delays = line
            .next()
            .ok_or("No number of delays")?
            .parse::<usize>()
            .map_err(|e| format!("Invalid number of delays: {}", e))?;
        let mut delays = Vec::with_capacity(n_delays);
        for _ in 0..n_delays {
            delays.push(
                line.next()
                    .ok_or("No delay")?
                    .parse::<Delay>()
                    .map_err(|e| format!("Invalid delay: {}", e))?,
            );
        }
        Ok(Self(FedId(fed_id), delays))
    }
}

impl ConnInfo {
    pub fn from_strs(rti: &str, federates: &[String]) -> Self {
        let stdp2d = SrcDestPair2Delay::from_str(rti).expect("Invalid RTI");
        let mut fed2uds = Vec::with_capacity(federates.len());
        for fed in federates {
            fed2uds.push(Fed2UpstreamDelays::from_str(fed).expect("Invalid federate"));
        }
        Self { stdp2d, fed2uds }
    }
    pub fn n_federates(&self) -> usize {
        self.stdp2d.1
    }
    pub fn get(&self, src: FedId, dest: FedId) -> Option<&Delay> {
        self.stdp2d.0.get(&(src, dest))
    }
    pub fn min_delays2dest(&self, dest: FedId) -> impl Iterator<Item = (&FedId, &Delay)> {
        self.stdp2d
            .0
            .iter()
            .filter(move |((_, d), _)| d.0 == dest.0)
            .map(|((s, _), d)| (s, d))
    }
    pub fn delays_in(&self, dest: FedId) -> impl Iterator<Item = Delay> + '_ {
        self.fed2uds[dest.0 as usize].1.iter().cloned()
    }
    pub fn delays_out(&self, src: FedId) -> impl Iterator<Item = (FedId, &Delay)> {
        self.fed2uds
            .iter()
            .filter(move |dest| self.stdp2d.0.get(&(src, dest.0)).is_some())
            .flat_map(|fed2uds| std::iter::repeat(fed2uds.0).zip(fed2uds.1.iter()))
    }
}
