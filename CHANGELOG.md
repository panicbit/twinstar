# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `document` API for creating Gemini documents
- preliminary timeout API, incl a special case for complex MIMEs by [@Alch-Emi](https://github.com/Alch-Emi)
- `Response::success_with_body` by [@Alch-Emi](https://github.com/Alch-Emi)
- `redirect_temporary_lossy` for `Response` and `ResponseHeader`
- `bad_request_lossy` for `Response` and `ResponseHeader`
- support for a lot more mime-types in `guess_mime_from_path`, backed by the `mime_guess` crate
- customizable TLS cert & key paths by [@Alch-Emi](https://github.com/Alch-Emi)
- `server_dir` default feature for serve_dir utils [@Alch-Emi](https://github.com/Alch-Emi)
### Improved
- build time and size by [@Alch-Emi](https://github.com/Alch-Emi)
### Changed
- Added route API [@Alch-Emi](https://github.com/Alch-Emi)

## [0.3.0] - 2020-11-14
### Added
- `GEMINI_MIME_STR`, the `&str` representation of the Gemini MIME
- `Meta::new_lossy`, constructor that never fails
- `Meta::MAX_LEN`, which is `1024`
- "lossy" constructors for `Response` and `Status` (see `Meta::new_lossy`)

### Changed
- `Meta::new` now rejects strings exceeding `Meta::MAX_LEN` (`1024`)
- Some `Response` and `Status` constructors are now infallible
- Improve error messages

### Deprecated
- Instead of `gemini_mime()` use `GEMINI_MIME`

## [0.2.0] - 2020-11-14
### Added
- Access to client certificates by [@Alch-Emi](https://github.com/Alch-Emi)
