import { Link } from '@tanstack/react-router';
import { m } from '../../../../paraglide/messages';

export const MfaLinks = () => {
  return (
    <div className="links">
      <Link to="/auth/mfa/recovery">
        <span>
          {m.login_mfa_use_instead({
            method: m.login_mfa_alternative_recovery(),
          })}
        </span>
      </Link>
      <Link to="/auth/mfa/totp">
        <span>
          {m.login_mfa_use_instead({
            method: m.login_mfa_alternative_totp(),
          })}
        </span>
      </Link>
      <Link to="/auth/login">
        <span>
          {m.login_mfa_use_instead({
            method: m.login_mfa_alternative_back(),
          })}
        </span>
      </Link>
    </div>
  );
};
