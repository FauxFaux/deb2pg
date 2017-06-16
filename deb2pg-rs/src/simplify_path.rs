use std::collections::HashMap;
use std::mem;

use std::borrow::Borrow;
use std::rc::Rc;

#[derive(Debug)]
enum Node {
    Dir(HashMap<String, Rc<Node>>),
    File(usize),
}

fn to_tree(input: Vec<Vec<String>>) -> HashMap<String, Node> {
    let mut root = Rc::new(Node::Dir(HashMap::new()));

    for (idx, paths) in input.iter().enumerate() {
        let mut curr = root;
        let mut paths = paths.clone();
        let last = paths.pop().unwrap();
        for path in paths {
            if let &Node::Dir(ref mut map) = curr.borrow() {
                curr = map.entry(path).or_insert_with(|| Rc::new(Node::Dir(HashMap::new()))).clone();
            } else {
                panic!("file is dir");
            }
        }

        if let &Node::Dir(mut map) = curr.borrow() {
            map.insert(last, Rc::new(Node::File(idx)));
        } else {
            panic!("file is dir");
        }
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