//! BLAKE3 hashing and domain-separated Merkle tree with inclusion proofs.

pub fn hash(bytes: &[u8]) -> [u8; 32] { *blake3::hash(bytes).as_bytes() }

fn hash_pair(l: &[u8;32], r: &[u8;32]) -> [u8;32] {
    let mut buf = [0u8; 65];
    buf[0] = 0x01; // node domain tag
    buf[1..33].copy_from_slice(l);
    buf[33..65].copy_from_slice(r);
    hash(&buf)
}
fn hash_leaf(l: &[u8;32]) -> [u8;32] {
    let mut buf = [0u8; 33];
    buf[0] = 0x00; // leaf domain tag
    buf[1..33].copy_from_slice(l);
    hash(&buf)
}

pub struct MerkleTree { levels: Vec<Vec<[u8;32]>> } // levels[0] = hashed leaves

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MerkleProof { pub index: usize, pub siblings: Vec<[u8;32]> }

impl MerkleTree {
    pub fn build(leaves: &[[u8;32]]) -> Self {
        assert!(!leaves.is_empty());
        let mut levels = vec![leaves.iter().map(hash_leaf).collect::<Vec<_>>()];
        while levels.last().unwrap().len() > 1 {
            let cur = levels.last().unwrap();
            let mut next = Vec::with_capacity((cur.len()+1)/2);
            let mut i = 0;
            while i < cur.len() {
                let l = cur[i];
                let r = if i+1 < cur.len() { cur[i+1] } else { cur[i] }; // duplicate last
                next.push(hash_pair(&l, &r));
                i += 2;
            }
            levels.push(next);
        }
        MerkleTree { levels }
    }
    pub fn root(&self) -> [u8;32] { *self.levels.last().unwrap().first().unwrap() }
    pub fn proof(&self, mut index: usize) -> MerkleProof {
        let orig = index;
        let mut siblings = Vec::new();
        for level in &self.levels[..self.levels.len()-1] {
            let sib = if index % 2 == 0 { (index+1).min(level.len()-1) } else { index-1 };
            siblings.push(level[sib]);
            index /= 2;
        }
        MerkleProof { index: orig, siblings }
    }
}

pub fn verify_proof(root: &[u8;32], leaf: &[u8;32], proof: &MerkleProof) -> bool {
    let mut h = hash_leaf(leaf);
    let mut idx = proof.index;
    for sib in &proof.siblings {
        h = if idx % 2 == 0 { hash_pair(&h, sib) } else { hash_pair(sib, &h) };
        idx /= 2;
    }
    &h == root
}

#[cfg(test)]
mod tests {
    use super::*;
    fn leaf(b: u8) -> [u8;32] { hash(&[b]) }

    #[test]
    fn proof_verifies_and_tamper_fails() {
        let leaves: Vec<[u8;32]> = (0..5u8).map(leaf).collect(); // non-power-of-two on purpose
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        for i in 0..leaves.len() {
            let p = tree.proof(i);
            assert!(verify_proof(&root, &leaves[i], &p), "index {i} should verify");
        }
        // wrong leaf fails
        let p0 = tree.proof(0);
        assert!(!verify_proof(&root, &leaf(99), &p0));
        // tampered sibling fails
        let mut p1 = tree.proof(1);
        p1.siblings[0][0] ^= 0xff;
        assert!(!verify_proof(&root, &leaves[1], &p1));
    }

    #[test]
    fn root_is_stable() {
        let leaves: Vec<[u8;32]> = (0..4u8).map(leaf).collect();
        assert_eq!(MerkleTree::build(&leaves).root(), MerkleTree::build(&leaves).root());
    }

    // New: a single-leaf tree — root is well-defined and proof verifies.
    #[test]
    fn single_leaf_tree() {
        let l = leaf(42);
        let tree = MerkleTree::build(&[l]);
        let root = tree.root();
        // root must equal hash_leaf(l) — only one level with one element
        let expected_root = {
            let mut buf = [0u8; 33];
            buf[0] = 0x00;
            buf[1..33].copy_from_slice(&l);
            hash(&buf)
        };
        assert_eq!(root, expected_root, "single-leaf root must be hash_leaf(leaf)");
        let p = tree.proof(0);
        assert!(verify_proof(&root, &l, &p), "single-leaf proof must verify");
    }

    // New: two-leaf tree — both proofs verify, wrong leaf for each fails.
    #[test]
    fn two_leaf_tree() {
        let l0 = leaf(10);
        let l1 = leaf(20);
        let tree = MerkleTree::build(&[l0, l1]);
        let root = tree.root();
        let p0 = tree.proof(0);
        let p1 = tree.proof(1);
        assert!(verify_proof(&root, &l0, &p0), "index 0 must verify");
        assert!(verify_proof(&root, &l1, &p1), "index 1 must verify");
        // cross-check: each proof rejects the other leaf
        assert!(!verify_proof(&root, &l1, &p0), "proof[0] must reject leaf[1]");
        assert!(!verify_proof(&root, &l0, &p1), "proof[1] must reject leaf[0]");
    }

    // New: power-of-two tree (8 leaves) — every index's proof verifies.
    #[test]
    fn power_of_two_tree_all_proofs_verify() {
        let leaves: Vec<[u8;32]> = (0..8u8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        for i in 0..leaves.len() {
            let p = tree.proof(i);
            assert!(verify_proof(&root, &leaves[i], &p), "index {i} must verify in 8-leaf tree");
        }
    }

    // New: a proof from index i rejected when checked against leaf j.
    #[test]
    fn wrong_index_proof_rejected() {
        let leaves: Vec<[u8;32]> = (0..8u8).map(leaf).collect();
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        let p3 = tree.proof(3);
        // every other leaf should fail against proof[3]
        for j in 0..8 {
            if j == 3 { continue; }
            assert!(!verify_proof(&root, &leaves[j], &p3),
                "proof[3] must reject leaf[{j}]");
        }
    }

    // New: leaf-vs-node domain separation — 2-leaf root differs from hashing the raw concatenation.
    #[test]
    fn domain_separation_leaf_vs_node() {
        let l0 = leaf(1);
        let l1 = leaf(2);
        let tree = MerkleTree::build(&[l0, l1]);
        let root = tree.root();
        // If there were no domain tags, an attacker could compute a fake "root" by
        // concatenating the raw leaf values directly. Verify that this differs.
        let raw_concat = {
            let mut buf = Vec::new();
            buf.extend_from_slice(&l0);
            buf.extend_from_slice(&l1);
            hash(&buf)
        };
        assert_ne!(root, raw_concat,
            "domain-separated root must differ from raw concatenation hash");
    }
}
