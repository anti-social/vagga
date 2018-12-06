use std::boxed::Box;
use std::fs::create_dir;
use std::error::Error;
use std::path::Path;
use std::str::FromStr;

use dkregistry::mediatypes::MediaTypes;
use dkregistry::errors::Error as DKError;
use dkregistry::reference::Reference;
use dkregistry::render::unpack as unpack_blobs;
use dkregistry::v2::Client as DKClient;
use dkregistry::v2::manifest::{ManifestSchema1Signed, ManifestSchema2};

use futures::Future;
use futures::prelude::*;

use quire::validate as V;

use tokio_core::reactor::Core;

use build_step::{BuildStep, VersionError, StepError, Digest, Config, Guard};
use container::root::temporary_change_root;

const DEFAULT_REGISTRY_HOST: &str = "registry-1.docker.io";

#[derive(Serialize, Deserialize, Debug)]
pub struct DockerImage {
    pub image: String,
    pub version: String,
}

impl DockerImage {
    pub fn config() -> V::Structure<'static> {
        V::Structure::new()
        .member("image", V::Scalar::new())
        .member("version", V::Scalar::new().default("latest"))
    }
}

impl BuildStep for DockerImage {
    fn name(&self) -> &'static str { "DockerImage" }

    fn hash(&self, _cfg: &Config, hash: &mut Digest) -> Result<(), VersionError> {
        hash.field("image", &self.image);
        hash.field("version", &self.version);
        Ok(())
    }

    fn build(&self, _guard: &mut Guard, build: bool) -> Result<(), StepError> {
        if build {
            download_image(DEFAULT_REGISTRY_HOST, &self.image, &self.version)?;
        }
        Ok(())
    }

    fn is_dependent_on(&self) -> Option<&str> { None }
}

fn download_image(registry_host: &str, image: &str, version: &str) 
-> Result<(), Box<Error>> {
    let mut tcore = Core::new()?;
    let mut dkclient = DKClient::configure(&tcore.handle())
        .registry(registry_host)
        .insecure_registry(false)
        .build()?;

    let login_scope = format!("repository:{}:pull", image);

    let blob_futures = authenticate_client(&mut dkclient, &login_scope)
        .and_then(|dkclient| {
            dkclient.has_manifest(image, version, None)
                .and_then(move |media_type| {
                    match media_type {
                        Some(t) => Ok((dkclient, t)),
                        None => Err(format!("Missing a manifest for {}:{}", image, version).into())
                    }    
                })
        })
        .and_then(|(dkclient, media_type)| {
            dkclient.get_manifest(image, version)
                .and_then(move |manifest_body| {
                    match fetch_layers(media_type, &manifest_body) {
                        Ok(layers) => Ok((dkclient, layers)),
                        Err(e) => Err(format!("{}", e).into()),
                    }
                })
        })
        .and_then(move |(dkclient, layers)| {
            futures::stream::iter_ok::<_, DKError>(layers)
                .and_then(move |layer| {
                    let blob_future = dkclient.get_blob(image, &layer);
                    blob_future.inspect(move |blob| {
                        info!("Layer {}, got {} bytes", &layer, blob.len());
                    })
                })
                .collect()
        });

    let blobs = match tcore.run(blob_futures) {
        Ok(blobs) => blobs,
        Err(e) => return Err(Box::new(e)),
    };

    let root_path = Path::new("/vagga/root");
    let r = unpack_blobs(&blobs, &root_path)?;

    Ok(())
}

pub fn authenticate_client<'a>(
    client: &'a mut DKClient,
    login_scope: &'a str,
) -> impl Future<Item = &'a DKClient, Error = DKError>
{
    futures::future::ok::<_, DKError>(client)
        .and_then(|dclient| {
            dclient.is_v2_supported().and_then(|v2_supported| {
                if !v2_supported {
                    Err("API v2 not supported".into())
                } else {
                    Ok(dclient)
                }
            })
        }).and_then(|dclient| {
            dclient.is_auth(None).and_then(|is_auth| {
                if is_auth {
                    Err("No login performed, but already authenticated".into())
                } else {
                    Ok(dclient)
                }
            })
        }).and_then(move |dclient| {
            dclient.login(&[&login_scope]).and_then(move |token| {
                dclient
                    .is_auth(Some(token.token()))
                    .and_then(move |is_auth| {
                        if !is_auth {
                            Err("Login failed".into())
                        } else {
                            Ok(dclient.set_token(Some(token.token())))
                        }
                    })
            })
        })
}

fn fetch_layers(media_type: MediaTypes, manifest_body: &Vec<u8>) 
-> Result<Vec<String>, Box<Error>> {
    match media_type {
        MediaTypes::ManifestV2S1Signed => {
            let m: ManifestSchema1Signed = match serde_json::from_slice(manifest_body.as_slice()) {
                Ok(json) => json,
                Err(e) => return Err(e.into())
            };
            Ok(m.get_layers())
        },
        MediaTypes::ManifestV2S2 => {
            let m: ManifestSchema2 = match serde_json::from_slice(manifest_body.as_slice()) {
                Ok(json) => json,
                Err(e) => return Err(e.into()),
            };
            Ok(m.get_layers())
        },
        t => {
            Err(format!("Unsupported manifest type: {:?}", t).into())
        },
    }
}