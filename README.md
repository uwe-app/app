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

## Editor

To develop the editor user interface use a release build of `uwe` to set up a live reload server and run the `editor` command with `debug_assertions`:

```
# Create a live reload development server
uwe dev --headless editor
# Show the editor native window
cargo run -- editor
```

Now as you make changes to the files in the `editor/site` folder the native window interface will automatically reload.

## Name Servers

To transfer a domain to our managed hosting service customers should update the name servers for the domain name with their registrar to these values:

```
ns1.uwe.app
ns2.uwe.app
ns3.uwe.app
ns4.uwe.app
```

> Some registrars may require a fully qualified name using a trailing period, eg: `ns1.uwe.app.` others prefer it without the trailing period - it depends on the registrar.

Until the customer-facing service is deployed we can create all the resources necessary for hosting using the private `web-host` executable.

The credentials (`<credentials>`) used in the `web-host` command examples should have full access to ACM, S3, Cloudfront and Route53.

To verify the name servers for a domain name have been configured and propagated run:

```
web-host ensure domain <domain-name>
```

An example using an IDNA internationalized domain name:

```
web-host ensure domain exämple.com
```

After the name servers for the domain have propagated follow the instructions in [Web Host](#web-host) to create all the resources to host the domain name.

## Web Host

Web hosts follow best practices for performance and security:

* Use a global CDN for edge locations near to clients
* Automatic compression for better response times
* Always use HTTPS for more secure file transfers

To setup all the resources for a host create a configuration file like this one (`uwe.app.toml`):

```toml
domain-name = "uwe.app"
bucket-name = "uwe.app"
region = ["ap-southeast-1"]
redirect-bucket-name = "www.uwe.app"
subject-alternative-names = ["*.uwe.app"]
```

Then run the command:

```
web-host ensure website uwe.app.toml --credentials=<credentials>
```

Creating a web host will:

* Verify name servers have propagated
* Create a hosted zone for the domain name
* Create an SSL certificate for the domain name and all sub-domains
* Create a bucket for the website files
* Create a bucket for the `www` redirect
* Create a CDN using the SSL certificate
* Configure the DNS records for the CDN
* Redirect all HTTP requests to HTTPS
* Configure the DNS for the `www` redirect

For fine-grained control of [resources](#resources) see the next section.

## Resources

It is recommended to use the `ensure` command whenever possible but sometimes it may be necessary to manage resources individually.

Replace `<credentials>` with the identifier of the AWS credentials and `<region>` with the target region, eg: `ap-southeast-1`. Replace all instances of `example.com` with the domain name that is being hosted.

The output of these commands will include identifiers for the created resources that you can take note of or find them later in the AWS console.

### Create or update a Hosted Zone

Create a hosted zone so that we can manage the DNS for the domain name:

```
web-host dns zone \
  --credentials=<credentials> \
  upsert \
  example.com
```

Take note of the hosted zone identifier which will be used later to update the DNS records.

### Create an SSL certificate

Create an SSL certificate for the domain name and all sub-domains; certificates are created in the US East (N Virginia) region which is required for usage with Cloudfront.

* `<zone-id>` The identifier for the Route53 hosted zone (eg: `Z0401662281B83ZUV01IN`).

After a certificate is created poll for the DNS records which can be used to authenticate domain ownership and automatically add them to the hosted zone created in the previous step (`<zone-id>`).

Once the DNS records for proving domain ownership have been added monitor the certificate status waiting for a `SUCCESS` status to indicate that the SSL certificate has been issued and can be used. The timeout for monitoring certificate status is 5 minutes by default use the `--timeout` option if necessary.

```
web-host cert issue \
  --credentials=<credentials> \
  --zone-id=<zone-id> \
  --alternative-name="*.example.com" \
  --monitor \
  example.com
```

Note the wildcard sub-domain `*.example.com` is quoted so the shell does not treat it as a glob pattern.

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

The CDN was created with IPv6 enabled so we should also create an `AAAA` record for IPv6 support:

```
web-host dns record \
  --zone-id=<zone-id> \
  --credentials=<credentials> \
  upsert \
  --cdn \
  --type=AAAA \
  example.com \
  <distribution-domain-name>
```

### Create the Redirect CNAME Record

So that all requests to `www.example.com` are redirected to `example.com`.

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

Be sure to check the `www` redirects too.

## Releases

A private executable `release-manager` performs all the steps for a release.

1) Publish a new release: `cargo run --bin=release-manager`.
2) Bump the version in `Cargo.toml` to prepare for the next release.

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

## Linux

For webview support install `libwebkit2gtk`:

```
sudo apt install libwebkit2gtk-4.0-dev
```

## Cross Compiling

MacOs minimum supported version: `10.10` (Yosemite).
MacOs SDK version: `11.1`

To cross-compile from Linux for MacOs the [osxcross][] library is required and you must add the `target/bin` directory to your `PATH`; see `.cargo/config` and `Makefile` for more details on the required executables.

To setup [osxcross][] for Linux follow these steps:

1) Clone the [osxcross][] repo
2) Install all the system dependencies described in packaging on linux plus `libbx2-dev` which is required for `xar` to work
3) Download the Xcode `xip` file, eg: `Xcode_12.4.xip`
4) Generate the SDK according to the [osxcross][] instructions (`./tools/gen_sdk_package_pbzx.sh Xcode_12.4.xip`) - you will need a lot of free disc space!
5) Move the generated tarball `MacOSX11.1.sdk.tar.xz` into the tarballs folder
6) Build the SDK target `UNATTENDED=yes ./build.sh`
7) Add the `target/bin` directory to `PATH` so the cross-compiler executables are available

### Dynamic Libraries

Use `objdump` on Linux to view the linked `.so` files:

```
objdump -p target/debug/uwe
```

Use `otool` to show dynamically linked libraries for MacOs, eg:

```
x86_64-apple-darwin20.2-otool -L target/x86_64-apple-darwin/release/uwe
```

## Preferences

This component is currently in limbo but may be restored in the future.

## Name Server Details

* Hosted Zone: `uwe.app`
* Hosted Zone ID: `Z04911223AOWXLH2LXWQ8`
* Delegation Set ID: `N02886841KKW7QD2MZLTC`

To get the IP addresses for a name server run:

```
dig A ns-544.awsdns-04.net +short
dig AAAA ns-544.awsdns-04.net +short
```

### ns1.uwe.app

* ns-544.awsdns-04.net
* 205.251.194.32
* 2600:9000:5302:2000::1

### ns2.uwe.app

* ns-2016.awsdns-60.co.uk
* 205.251.199.224
* 2600:9000:5307:e000::1

### ns3.uwe.app

* ns-507.awsdns-63.com
* 205.251.193.251
* 2600:9000:5301:fb00::1

### ns4.uwe.app

* ns-1518.awsdns-61.org
* 205.251.197.238
* 2600:9000:5305:ee00::1

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
