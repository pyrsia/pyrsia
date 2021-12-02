use super::model::package::{PackageVersion, Package};

//TO BE IMPLEMENETD
//get_package: provided package_name it returns all the metadata for that package version
pub fn get_package(pkg_name:String) -> Result<Package, anyhow::Error> {
    let pkg = Package{ ..Default::default() };
    Ok(pkg)

}

//TO BE IMPLEMENETD
//get_package_version: provided id and package_version it returns all the metadata for that package version
pub fn get_package_version(id:String, pkg_ver: String) -> Result<PackageVersion, anyhow::Error> {
    let pkg_ver = PackageVersion{ ..Default::default() };
    Ok(pkg_ver)
}