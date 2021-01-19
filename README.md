# Universal Web Editor

Repositories must be siblings, for example:

```
.
├── app
├── blueprints
├── community
├── documentation
├── integrations
├── library
├── plugins
├── registry
├── releases
├── sites
├── syntax
└── syntax-compiler
```

* [app][]: Main source code (this repository).
* [blueprints][]: Project blueprints.
* [community][]: Community discussions.
* [integrations][]: Javascript and CSS bundler integrations.
* [library][]: Additional libraries; search runtime and third-party forks.
* [plugins][]: Source code for the plugins.
* [registry][]: Index of available plugins.
* [releases][]: Manifest of platform releases.
* [sites/blog][blog]: UWE blog.
* [site/website][website]: UWE website.
* [syntax][]: Syntax highlighting language definitions binary files.
* [syntax-compiler][]: Compiles the syntax highlighting definitions.

## Web Hosts

Until the customer-facing service is deployed we can create all the resources necessary for hosting using the private `web-host` executable. This does not cover updating name servers and creating SSL certificates at this stage.

You should replace `<credentials>` with the identifier of the AWS credentials and `<region>` with the target region, eg: `ap-southeast-1`. Replace all instances of `example.com` with the domain name that is being hosted.

The credentials should have full access to S3, Cloudfront and Route53. The output of these commands will include identifiers for the created resources that you can take note of or find them later in the AWS console.

### Create a Hosted Zone

Create a hosted zone so that we can manage the DNS for the domain name:

```
web-host dns zone --credentials=<credentials> create example.com
```

The output will include a list of name servers for the hosted zone and the owner of the domain needs to update their name servers with the domain registrar.

Take note of the hosted zone identifier which will be used later to update the DNS records.

### Create an SSL certificate

TODO

### Create a Bucket

Create a bucket called `example.com`:

```
web-host bucket up \
  --credentials=<credentials> \
  --region=<region> \
  example.com
```

### Create a Redirect Bucket

This redirects all requests from `www.example.com` bucket to the `example.com` domain name:

```
web-host bucket up \
  --credentials=<credentials> \
  --region=<region> \
  --redirect-host-name=example.com \
  www.example.com
```

Take a note of the domain name for the endpoint so we can configure a `CNAME` record later, eg: `www.example.com.s3-website-ap-southeast-1.amazonaws.com` - the actual value for the endpoint will vary by region.

### Create a CDN

Create a content distribution network:

* `<acm-certificate-arn>` The SSL certificate ARN created in ACM (eg: `arn:aws:acm:us-east-1:077333142038:certificate/0e3e9184-ca15-42b7-9b3d-4591487d3b30`). The certificate **must** include the domain name `example.com`.
* `<s3-bucket-endpoint>` The S3 bucket endpoint returned when the bucket was created, should include a leading `http://` scheme (eg: `http://example.com.s3-website-ap-southeast-1.amazonaws.com`).

```
web-host cdn create \
  --credentials=<credentials> \
  --origin-id=example-com \
  --alias=example.com \
  --protocol-policy=redirect-to-https \
  --acm-certificate-arn=<acm-certificate-arn> \ 
  <s3-bucket-endpoint>
```

### Create the DNS Record(s)

Create the DNS alias record for the hosted zone pointing to the cloudfront distribution domain name:

* `<zone-id>` The identifier for the Route53 hosted zone (eg: `Z0401662281B83ZUV01IN`).
* `<distribution-domain-name>` The domain name for the cloudfront CDN distribution (eg: `dwi0e9r7b599c.cloudfront.net`).

```
web-host dns record \
  --zone-id=<zone-id> \
  --credentials=<credentials> \
  upsert \
  --cdn \
  --type=A \
  example.com \
  <distribution-domain-name>
```

### TODO: create an ipv6 AAAA record

### Create the Redirect CNAME Record

So that all requests to `www.example.com` and redirected to `example.com`.

* `<zone-id>` The identifier for the Route53 hosted zone (eg: `Z0401662281B83ZUV01IN`).
* `<s3-website-domain-name>` The domain name for the redirect bucket endpoint created earlier (eg: `www.example.com.s3-website-ap-southeast-1.amazonaws.com`).

```
web-host dns record \
  --zone-id=<zone-id> \
  --credentials=<credentials> \
  upsert \
  --type=CNAME \
  www.example.com \
  <s3-website-domain-name>
```

## Verify

You may need to wait a while depending upon the propagation status of the name servers and the status of the CDN, afterwards the hosting can be verified using `dig` and `curl`:

```
dig example.com
curl -L example.com
```

## Releases

A private executable `release-manager` performs all the steps for a release.

1) Bump the version in `Cargo.toml`.
2) Publish a new release: `cargo run --bin=release-manager`.

If you need them `release-manager` supports `--force` to force overwrite an existing version and `--skip-build` if you know that the release artifacts are up to date. If uploads succeed but the release fails afterwards `--skip-upload` can also be used. These flags are primarily used for testing and development purposes; for example if you encounter a network error after a build has succeeded you could use:

```
cargo run --bin=release-manager-- --force --skip-build
```

Be aware force overwriting can cause a checksum mismatch when Cloudfront serves a stale executable version so you should invalidate the Cloudfront distribution.

## Versioning

Multiple versions installed by `uvm` are accessed via the shim executable `uwe-shim` and `upm-shim` which are installed into the installation `bin` directory and determine the version to execute.

They search the current directory and parents for a `.uwe-version` files containing a valid semver which if present is used otherwise they defer to the default version (selected using `uvm use`).

Because `uwe` can also accept paths to projects other than the current working directory and the shim executables have no knowledge of these project path arguments; it must also check whether a switch is needed once it has received a project path. This incurs additional overhead so the search for local versions in this situation should only happen when the project path is not equal to the current working directory as we know that if the project path is the current working directory the shim should already have resolved any local version file.

Running `uvm ls` should also mark an installation as comming from a version file; it can call `release::find_local_version()` to try to find a local file.

## Uninstall

To remove an installation run `cargo run --bin=uvm -- uninstall`.

## Install

To test an installation using the quick install script:

```
curl https://releases.uwe.app/install.sh | sh
```

## Plugins

Plugin publishing is restricted to those that have access to the s3 bucket and the [registry][] repository; to publish plugins during the alpha and beta phases certain environment variables need to be set:

```
export UPM_PUBLISH="$HOME/path/to/registry/folder"
export UPM_PUBLISH_REPO="$HOME/path/to/repo"
export UPM_PUBLISH_PROFILE="..."
export UPM_PUBLISH_REGION="ap-southeast-1"
export UPM_PUBLISH_BUCKET="..."
```

It is **required** to set the `pushurl` and `push` refspec:

```
[remote "origin"]
  url = git@github.com:uwe-app/registry.git
  fetch = +refs/heads/*:refs/remotes/origin/*
  pushurl = git@github.com:uwe-app/registry.git
  push = refs/heads/main:refs/heads/main
```

## Cross Compiling

To cross-compile from Linux for MacOs the [osxcross][] library is required and you must add the `bin` directory to your `PATH`; see `.cargo/config` and `Makefile` for more details.

## Preferences

This component is currently in limbo but may be restored in the future.

## Notes

Additional information some of which may be obsolete in [NOTES](/NOTES.md).

[app]: https://github.com/uwe-app/app
[blueprints]: https://github.com/uwe-app/blueprints
[community]: https://github.com/uwe-app/community
[integrations]: https://github.com/uwe-app/integrations
[library]: https://github.com/uwe-app/library
[plugins]: https://github.com/uwe-app/plugins
[registry]: https://github.com/uwe-app/registry
[releases]: https://github.com/uwe-app/releases
[blog]: https://github.com/uwe-app/blog
[website]: https://github.com/uwe-app/website
[syntax]: https://github.com/uwe-app/syntax
[syntax-compiler]: https://github.com/uwe-app/syntax-compiler

[osxcross]: https://github.com/tpoechtrager/osxcross
