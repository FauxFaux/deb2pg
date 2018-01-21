use fapt_pkg;

use errors::*;

use connect;
use Package;

pub fn incomplete_packages(mirror: &str) -> Result<Vec<Package>> {
    let mut fapt = fapt_pkg::System::cache_dirs_only("lists")?;

    fapt.add_sources_entry_line(&format!("deb-src {} stretch main contrib non-free", mirror))?;
    fapt.add_keyring_paths(&["/usr/share/keyrings/debian-archive-keyring.gpg"])?;
    fapt.update()?;

    let mut packages = Vec::new();
    let exists_conn = connect()?;
    let exists_tran = exists_conn.transaction()?;
    let exists_stat = exists_tran.prepare("SELECT EXISTS(SELECT 1 FROM container WHERE info=$1)")?;
    for package in find_packages(&fapt)? {
        if exists_stat.query(&[&package.container()])?.get(0).get(0) {
            continue;
        }

        packages.push(package);
    }

    Ok(packages)
}

fn find_packages(fapt: &fapt_pkg::System) -> Result<Vec<Package>> {
    let mut ret = Vec::new();
    fapt.walk_sections(|map| {
        let pkg = map.get_if_one_line("Package").ok_or("invalid Package")?;
        let version = map.get_if_one_line("Version").ok_or("invalid Version")?;
        ret.push(Package {
            prefix: format!("{}/{}_{}", subdir(&pkg), pkg, version),

            pkg: pkg.to_string(),
            version: version.to_string(),
            dir: map.get_if_one_line("Directory")
                .ok_or("invalid Directory")?
                .to_string(),

            dsc: map.as_ref()["Files"]
                .iter()
                .filter(|line| line.ends_with(".dsc"))
                .next()
                .unwrap()
                .split_whitespace()
                .nth(2)
                .unwrap()
                .to_string(),

            size: map.as_ref()["Files"]
                .iter()
                .map(|line| {
                    let num: &str = line.split_whitespace().nth(1).unwrap();
                    let num: u64 = num.parse().unwrap();
                    num
                })
                .sum(),
        });
        Ok(())
    })?;
    Ok(ret)
}


// Sigh, I've already written this.
fn subdir(name: &str) -> &str {
    if name.starts_with("lib") {
        &name[..4]
    } else {
        &name[..1]
    }
}