use std::cmp;

use std::collections::HashMap;

use std::collections::hash_map;

#[derive(Debug)]
enum Node {
    Dir(HashMap<String, Node>),
    File(usize)
}

///    >>> shortest_match("foo", "bar")
///    ''
///    >>> shortest_match("foo", "food")
///    'foo'
///    >>> shortest_match("abcd", "abfd")
///    'ab'
fn shortest_match(left: &str, right: &str) -> String {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let shortest = cmp::min(left.len(), right.len());
    let mut i = 0;

    while i < shortest {
        if left[i] != right[i] {
            break;
        }

        i += 1;
    }

    (&left[0..i]).iter().collect()
}

///    >>> find_prefix(["foo/bar", "foo/baz"])
///    'foo/'
///    >>> find_prefix([])
///    ''
///    >>> find_prefix(['one/two'])
///    'one/'
///    >>> find_prefix(['one/two', 'one'])
///    ''
///    >>> find_prefix(['one/two', 'two'])
///    ''
fn find_prefix(within: Vec<String>) -> String {

    let mut it = within.iter();
    let empty_string = "".to_string();
    let mut prefix: String = it.next().unwrap_or(&empty_string).to_string();
    for item in it {
        if item.starts_with(prefix.as_str()) {
            continue
        }
        prefix = shortest_match(prefix.as_str(), item);
    }

    match prefix.rfind('/') {
        None => "",
        Some(slash) => &prefix[0..slash + 1],
    }.to_string()
}

fn strip_prefix<T>(off_of: HashMap<String, T>) -> (String, HashMap<String, T>) {
    let prefix = find_prefix(off_of.keys().map(|s| s.to_string()).collect());

    if prefix.is_empty() {
        (prefix, off_of)
    } else {
        let mut new_val = HashMap::with_capacity(off_of.len());
        for (k, v) in off_of {
            if k != prefix {
                new_val.insert((&k[prefix.len()..]).to_string(), v);
            }
        }
        (prefix, new_val)
    }
}


fn fixup_path_internal(
        structure: HashMap<String, Node>,
        so_far: &[String]) -> Vec<(Vec<String>, usize)> {
//    >>> list(fixup_path_internal({'a': {'b/c': 3, 'b/d': 4}}, []))
//    [(['a', 'b/', 'c'], 3), (['a', 'b/', 'd'], 4)]

    let mut ret = Vec::new();


    for (item, sub) in structure {
        let mut next: Vec<String> = so_far.iter().map(|s| s.to_string()).collect();

        match sub {
            Node::File(pos) => {
                next.push(item);
                ret.push((next, pos))
            },
            Node::Dir(sub) => {
                let (prefix, sub) = strip_prefix(sub);
                next.push(item);
                if !prefix.is_empty() {
                    next.push(prefix);
                }
                for value in fixup_path_internal(sub, &next) {
                    ret.push(value);
                }
            }
        }
    }

    ret
}


fn fixup_path(structure: HashMap<String, Node>)
        -> Vec<(Vec<String>, usize)> {
//    >>> list(fixup_path({'a/b': 3, 'a/c': 4}))
//    [(['a/', 'b'], 3), (['a/', 'c'], 4)]

    let (prefix, sub) = strip_prefix(structure);
    if prefix.is_empty() {
        fixup_path_internal(sub, &[])
    } else {
        fixup_path_internal(sub, &[prefix.to_string()])
    }
}


pub fn simplify(input: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let tree = list_to_tree(input);
    let mut fixed = fixup_path(tree);
    fixed.sort_by_key(|x| x.1);
    fixed.into_iter().map(|x| x.0).collect()
}


fn add(into: &mut HashMap<String, Node>, remaining: &[String], pos: usize) {
    match remaining.len() {
        0 => unreachable!(),
        1 => { into.insert(remaining[0].to_string(), Node::File(pos)); }
        _ => {
            match into.entry(remaining[0].to_string()) {
                hash_map::Entry::Occupied(mut exists) => {
                    if let &mut Node::Dir(ref mut map) = exists.get_mut() {
                        add(map, &remaining[1..], pos);
                    }
                }
                hash_map::Entry::Vacant(vacant) => {
                    let mut map = HashMap::new();
                    add(&mut map, &remaining[1..], pos);
                    vacant.insert(Node::Dir(map));
                }
            }
        }
    }
}

fn list_to_tree(from: Vec<Vec<String>>) -> HashMap<String, Node> {
    let mut root = HashMap::new();
    for (pos, item) in from.into_iter().enumerate() {
        add(&mut root, &item, pos);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn trial() {
        for out in simplify(to_vec(&[
            &["some.dsc"],
            &["foo.tar", "foo-1337/"],
            &["foo.tar", "foo-1337/Makefile"],
            &["foo.tar", "foo-1337/src/main.c"],
            &["foo.tar", "foo-1337/src/help.h"],
            &["foo.tar", "foo-1337/some.jar", "MANIFEST.MF"],
            &["other.dsc"]
        ])) {
            println!("{:?}", out);
        }
    }

    fn to_vec(what: &[&[&str]]) -> Vec<Vec<String>> {
        what.iter().map(|inner|
            inner.iter().map(|x| x.to_string()).collect::<Vec<String>>()
        ).collect::<Vec<Vec<String>>>()
    }
}