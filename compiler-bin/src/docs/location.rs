use std::path::PathBuf;

use documentation::schema::Location;

use crate::docs::PursLocation;

pub fn manifest_location(
    location: Option<PursLocation>,
    reference: Option<String>,
) -> Option<Location> {
    match location? {
        PursLocation::GitHub { owner, repository, subdir } => {
            Some(github_location(owner, repository, reference, subdir))
        }
        PursLocation::Git { url, subdir } => Some(location_from_git_url(&url, reference, subdir)),
    }
}

pub fn package_reference_location(reference: &spago::PackageReference) -> Option<Location> {
    let spago::PackageReference::Git { url: Some(url), rev, subdir, .. } = reference else {
        return None;
    };

    let reference = Some(rev.to_string());
    let subdir = subdir.as_ref().map(path_to_string);

    Some(location_from_git_url(url.as_str(), reference, subdir))
}

fn location_from_git_url(url: &str, reference: Option<String>, subdir: Option<String>) -> Location {
    let Some((owner, repository)) = github_repository_from_url(url) else {
        return Location::Git { url: url.to_string(), reference, subdir };
    };

    github_location(owner, repository, reference, subdir)
}

fn github_location(
    owner: String,
    repository: String,
    reference: Option<String>,
    subdir: Option<String>,
) -> Location {
    Location::GitHub {
        url: github_repository_url(&owner, &repository),
        owner,
        repository,
        reference,
        subdir,
    }
}

fn github_repository_from_url(url: &str) -> Option<(String, String)> {
    let url = url::Url::parse(url).ok()?;
    if url.host_str()? != "github.com" {
        return None;
    }

    let mut segments = url.path_segments()?;
    let owner = segments.next()?.to_string();
    let repository = segments.next()?.trim_end_matches(".git").to_string();

    if owner.is_empty() || repository.is_empty() {
        return None;
    }

    Some((owner, repository))
}

fn github_repository_url(owner: &str, repository: &str) -> String {
    format!("https://github.com/{owner}/{repository}")
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}
