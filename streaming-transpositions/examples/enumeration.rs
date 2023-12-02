use streaming_transpositions::{BigSmallIterator, OgRank};

fn sort(pairs: impl Iterator<Item = (u32, u32)>) -> Vec<(u32, u32)> {
    let mut pairs: Vec<_> = pairs.into_iter().collect();
    pairs.sort_by(|a, b| match a.0.cmp(&b.0) {
        std::cmp::Ordering::Equal => a.1.cmp(&b.1),
        other => other,
    });
    pairs
}

fn viz(pairs: impl Iterator<Item = (u32, u32)>) -> String {
    let mut ret = String::new();
    for (a, b) in pairs {
        for _ in 0..a {
            ret.push(' ');
        }
        ret.push(a.to_string().chars().next().unwrap());
        for _ in a + 1..b {
            ret.push(' ');
        }
        ret.push(b.to_string().chars().next().unwrap());
        ret.push('\n');
    }
    ret
}

fn naive(max_ogrank: u32) -> Vec<(u32, u32)> {
    let mut ret = Vec::new();
    for start in 0..max_ogrank - 1 {
        for other in start + 1..max_ogrank {
            if start != other {
                ret.push((start, other));
            }
        }
    }
    ret
}

fn sawtooth(max_ogrank: u32) -> Vec<(u32, u32)> {
    let mut ret = Vec::new();
    for power in 0..max_ogrank.ilog2() + 1 {
        let diffmax = (1 << (power + 1)).min(max_ogrank);
        let diffmin = 1 << power;
        for diff in (diffmin..diffmax).rev() {
            for start in 0..max_ogrank - diff {
                let other = start + diff;
                ret.push((start, other));
            }
        }
    }
    ret
}

fn start_first_sawtooth(max_ogrank: u32) -> Vec<(u32, u32)> {
    let max_ogrank = max_ogrank as i32;
    let mut ret = Vec::new();
    for power in 0..max_ogrank.ilog2() + 1 {
        let diffmaxstrict = (1 << (power + 1)).min(max_ogrank);
        let diffmin = 1 << power;
        for start_minus_diff in 0 - diffmaxstrict + 1..max_ogrank - diffmin {
            for diff in diffmin.max(-start_minus_diff)
                ..diffmaxstrict.min((max_ogrank - start_minus_diff) / 2 + 1)
            {
                let start = start_minus_diff + diff;
                if start + diff < max_ogrank {
                    ret.push((start as u32, (start + diff) as u32));
                }
            }
        }
    }
    ret
}

fn main() {
    println!("{:?}", sawtooth(8).into_iter());
    println!("{:?}", sort(sawtooth(8).into_iter()));
    let length = 9;
    println!("{:?}", start_first_sawtooth(length));
    println!("{}", viz(start_first_sawtooth(length).into_iter()));
    println!(
        "{}",
        viz(BigSmallIterator::new(OgRank(length)).map(|(a, b)| (a.0, b.0)))
    );
    println!("{:?}", sort(start_first_sawtooth(length).into_iter()));
    println!("{:?}", naive(length));
}
