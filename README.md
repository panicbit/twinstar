```
   __           _            __
  / /__      __(_)___  _____/ /_____ ______
 / __/ | /| / / / __ \/ ___/ __/ __ `/ ___/
/ /_ | |/ |/ / / / / (__  ) /_/ /_/ / /
\__/ |__/|__/_/_/ /_/____/\__/\__,_/_/
```

- [Documentation](https://docs.rs/twinstar)
- [GitHub](https://github.com/panicbit/twinstar)

# Usage

Add the latest version of twinstar to your `Cargo.toml`.

## Manually

```toml
twinstar = "0.4.0" # check crates.io for the latest version
```

## Automatically

```sh
cargo add twinstar
```

# Generating a key & certificate

Run
```sh
mkdir cert && cd cert
openssl req -x509 -nodes -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365
```
and enter your domain name (e.g. "localhost" for testing) as Common Name (CN).

Alternatively, if you want to include multiple domains add something like `-addext "subjectAltName = DNS:localhost, DNS:example.org"`.
