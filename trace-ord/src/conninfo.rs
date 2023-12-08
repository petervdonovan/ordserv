use std::{collections::HashMap, fmt::Display, ops::Add, str::FromStr};

#[derive(Debug, Clone, Copy)]
pub struct Delay(u64, u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag(pub i64, pub i64);

impl Add<Delay> for Tag {
    type Output = Self;

    fn add(self, rhs: Delay) -> Self::Output {
        let mut tag = self;
        tag.0 += rhs.0 as i64;
        tag.1 += rhs.1 as i64;
        tag
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FedId(pub i32);

pub struct ConnInfo(pub HashMap<(FedId, FedId), Delay>);

impl FromStr for ConnInfo {
    type Err = String;

    /// Format:
    ///   number_of_scheduling_nodes
    ///   (enclave_id num_upstream (upstream_federate_id upstream_delay)*\n)*
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut conn_info = HashMap::new();
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
                conn_info.insert((FedId(enclave_id), FedId(upstream_fed_id)), upstream_delay);
            }
            conn_info.insert((FedId(enclave_id), FedId(enclave_id)), Delay(0, 0));
        }
        Ok(Self(conn_info))
    }
}
