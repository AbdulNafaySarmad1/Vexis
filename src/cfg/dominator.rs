//! Iterative dominator computation (Cooper–Harvey–Kennedy, 2001).
//!
//! Operates on an explicit successor list so it can be run per-recovered-function
//! without materialising a petgraph subgraph.

use std::collections::{BTreeMap, HashMap};

/// Immediate-dominator map for a rooted subgraph.
pub struct Dominators {
    /// node -> immediate dominator (the root maps to itself).
    pub idom: HashMap<u64, u64>,
    /// reverse-postorder numbering used during the fix-point.
    pub rpo_index: HashMap<u64, usize>,
}

impl Dominators {
    /// `succ` maps each node to its successor nodes. Nodes not reachable from
    /// `entry` are omitted from the result.
    pub fn compute(entry: u64, succ: &BTreeMap<u64, Vec<u64>>) -> Dominators {
        let order = reverse_postorder(entry, succ);
        let mut rpo_index = HashMap::new();
        for (i, &n) in order.iter().enumerate() {
            rpo_index.insert(n, i);
        }

        // Predecessor list restricted to reachable nodes.
        let reachable: std::collections::HashSet<u64> = order.iter().copied().collect();
        let mut preds: HashMap<u64, Vec<u64>> = HashMap::new();
        for (&n, ss) in succ {
            if !reachable.contains(&n) {
                continue;
            }
            for &s in ss {
                if reachable.contains(&s) {
                    preds.entry(s).or_default().push(n);
                }
            }
        }

        let mut idom: HashMap<u64, Option<u64>> = order.iter().map(|&n| (n, None)).collect();
        idom.insert(entry, Some(entry));

        let mut changed = true;
        while changed {
            changed = false;
            for &b in order.iter().skip(1) {
                let bpreds = preds.get(&b).cloned().unwrap_or_default();
                let mut new_idom: Option<u64> = None;
                for p in bpreds {
                    if idom.get(&p).and_then(|x| *x).is_some() {
                        new_idom = Some(match new_idom {
                            None => p,
                            Some(cur) => intersect(cur, p, &idom, &rpo_index),
                        });
                    }
                }
                if let Some(ni) = new_idom {
                    if idom.get(&b).and_then(|x| *x) != Some(ni) {
                        idom.insert(b, Some(ni));
                        changed = true;
                    }
                }
            }
        }

        Dominators {
            idom: idom
                .into_iter()
                .filter_map(|(k, v)| v.map(|d| (k, d)))
                .collect(),
            rpo_index,
        }
    }

    pub fn dominates(&self, a: u64, b: u64) -> bool {
        let mut cur = b;
        loop {
            if cur == a {
                return true;
            }
            match self.idom.get(&cur) {
                Some(&d) if d != cur => cur = d,
                _ => return false,
            }
        }
    }
}

fn intersect(
    mut a: u64,
    mut b: u64,
    idom: &HashMap<u64, Option<u64>>,
    rpo: &HashMap<u64, usize>,
) -> u64 {
    while a != b {
        while rpo[&a] > rpo[&b] {
            a = idom[&a].unwrap();
        }
        while rpo[&b] > rpo[&a] {
            b = idom[&b].unwrap();
        }
    }
    a
}

fn reverse_postorder(entry: u64, succ: &BTreeMap<u64, Vec<u64>>) -> Vec<u64> {
    let mut visited = std::collections::HashSet::new();
    let mut post = Vec::new();
    let mut stack = vec![(entry, 0usize)];
    visited.insert(entry);
    while let Some((node, idx)) = stack.pop() {
        let succs = succ.get(&node).map(|v| v.as_slice()).unwrap_or(&[]);
        if idx < succs.len() {
            stack.push((node, idx + 1));
            let s = succs[idx];
            if visited.insert(s) {
                stack.push((s, 0));
            }
        } else {
            post.push(node);
        }
    }
    post.reverse();
    post
}
