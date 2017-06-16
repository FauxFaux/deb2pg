use std::collections::HashMap;
use std::mem;

use std::borrow::Borrow;
use std::cell::Cell;
use std::rc::Rc;

enum Node {
    Dir(Cell<HashMap<String, Node>>),
    File(usize),
}

fn to_tree(input: Vec<Vec<String>>) -> HashMap<String, Node> {
    let mut root: HashMap<String, Node> = HashMap::new();

    for (idx, paths) in input.iter().enumerate() {
        let mut curr: Cell<HashMap<String, Node>> = &mut root;

        let mut paths = paths.clone();
        let last = paths.pop().unwrap();
        for path in paths {
            if !curr.contains_key(&path) {
                curr.insert(path, Node::Dir(Cell::new(HashMap::new())));
            }

            mem::replace(&mut curr, match curr[&path] {
                Node::Dir(ref mut map) => map,
                _ => unreachable!(),
            });
        }

        curr.insert(last, Node::File(idx));
    }

    println!("{:?}", root);
    unimplemented!();
}

pub fn simplify(input: Vec<Vec<String>>) -> Vec<Vec<String>> {
    unimplemented!();
}

#[cfg(test)]
mod tests {
    use super::simplify;

    fn assert_identity(of: &[&[&str]]) {
        let val = to_vec(of);
        assert_eq!(val, simplify(val.clone()))
    }

    #[test]
    fn no_change() {
        assert_identity(&[
            &["a"],
            &["b"],
            &["c"],
        ]);
    }

    fn to_vec(what: &[&[&str]]) -> Vec<Vec<String>> {
        what.iter().map(|inner|
            inner.iter().map(|x| x.to_string()).collect::<Vec<String>>()
        ).collect::<Vec<Vec<String>>>()
    }
}