use std::cmp;
use std::mem;

use std::collections::HashMap;

pub fn simplify(mut input: Vec<Vec<String>>) -> Vec<Vec<String>> {
    input.sort_by(|left, right| {
        for i in 0..cmp::min(left.len(), right.len()) {
            match left[i].cmp(&right[i]) {
                cmp::Ordering::Equal => continue,
                otherwise => return otherwise,
            };
        }

        left.len().cmp(&right.len())
    });

    // a b/c d
    // a b/c e
    // g

    let mut previous = None;

//    let mut prefix: HashMap<Vec<String>, String> = HashMap::new();

    // first pass
    for paths in &input {
        if previous.is_none() {
            previous = Some(paths);
            continue;
        }

        let previous = previous.as_ref().unwrap();
    }

    input.clone()
}

fn diff_point<T: PartialEq>(left: &[T], right: &[T]) -> usize {
    let shortest = cmp::min(left.len(), right.len());
    for i in 0..shortest {
        if left[i] != right[i] {
            return i;
        }
    }

    shortest
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

    #[test]
    fn diff() {
        assert_eq!(2, diff_point(&[1, 2, 3], &[1, 2, 4]));
        assert_eq!(0, diff_point(&[1, 2, 3], &[5, 6]));
        assert_eq!(2, diff_point(&[1, 2, 3], &[1, 2]));
        assert_eq!(1, diff_point(&[1, 2, 3], &[1]));
        assert_eq!(3, diff_point(&[1, 2, 3], &[1, 2, 3]));
    }

    fn to_vec(what: &[&[&str]]) -> Vec<Vec<String>> {
        what.iter().map(|inner|
            inner.iter().map(|x| x.to_string()).collect::<Vec<String>>()
        ).collect::<Vec<Vec<String>>>()
    }
}