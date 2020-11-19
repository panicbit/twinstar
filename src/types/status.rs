#[derive(Debug,Copy,Clone,PartialEq,Eq)]
pub struct Status(u8);

impl Status {
    pub const INPUT: Self = Self(10);
    pub const SENSITIVE_INPUT: Self = Self(11);
    pub const SUCCESS: Self = Self(20);
    pub const REDIRECT_TEMPORARY: Self = Self(30);
    pub const REDIRECT_PERMANENT: Self = Self(31);
    pub const TEMPORARY_FAILURE: Self = Self(40);
    pub const SERVER_UNAVAILABLE: Self = Self(41);
    pub const CGI_ERROR: Self = Self(42);
    pub const PROXY_ERROR: Self = Self(43);
    pub const SLOW_DOWN: Self = Self(44);
    pub const PERMANENT_FAILURE: Self = Self(50);
    pub const NOT_FOUND: Self = Self(51);
    pub const GONE: Self = Self(52);
    pub const PROXY_REQUEST_REFUSED: Self = Self(53);
    pub const BAD_REQUEST: Self = Self(59);
    pub const CLIENT_CERTIFICATE_REQUIRED: Self = Self(60);
    pub const CERTIFICATE_NOT_AUTHORIZED: Self = Self(61);
    pub const CERTIFICATE_NOT_VALID: Self = Self(62);

    pub const fn code(&self) -> u8 {
        self.0
    }

    pub fn is_success(&self) -> bool {
        self.category().is_success()
    }

    pub const fn category(&self) -> StatusCategory {
        let class = self.0 / 10;

        match class {
            1 => StatusCategory::Input,
            2 => StatusCategory::Success,
            3 => StatusCategory::Redirect,
            4 => StatusCategory::TemporaryFailure,
            5 => StatusCategory::PermanentFailure,
            6 => StatusCategory::ClientCertificateRequired,
            _ => StatusCategory::PermanentFailure,
        }
    }
}

#[derive(Copy,Clone,PartialEq,Eq)]
pub enum StatusCategory {
    Input,
    Success,
    Redirect,
    TemporaryFailure,
    PermanentFailure,
    ClientCertificateRequired,
}

impl StatusCategory {
    pub fn is_input(&self) -> bool {
        *self == Self::Input
    }

    pub fn is_success(&self) -> bool {
        *self == Self::Success
    }

    pub fn redirect(&self) -> bool {
        *self == Self::Redirect
    }

    pub fn is_temporary_failure(&self) -> bool {
        *self == Self::TemporaryFailure
    }

    pub fn is_permanent_failure(&self) -> bool {
        *self == Self::PermanentFailure
    }

    pub fn is_client_certificate_required(&self) -> bool {
        *self == Self::ClientCertificateRequired
    }
}
