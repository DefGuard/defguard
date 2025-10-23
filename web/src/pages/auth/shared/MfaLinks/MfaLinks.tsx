import './style.scss';
import { Link } from '@tanstack/react-router';
import { m } from '../../../../paraglide/messages';
import { useAuth } from '../../../../shared/hooks/useAuth';

export const MfaLinks = () => {
  const mfa = useAuth((s) => s.mfaLogin);
  if (!mfa) return null;
  return (
    <div className="mfa-links">
      {mfa.totp_available && (
        <Link to="/auth/mfa/totp">
          <span>
            {m.login_mfa_use_instead({
              method: m.login_mfa_alternative_totp(),
            })}
          </span>
        </Link>
      )}
      {mfa.webauthn_available && (
        <Link to="/auth/mfa/webauthn">
          <span>
            {m.login_mfa_use_instead({
              method: m.login_mfa_alternative_passkey(),
            })}
          </span>
        </Link>
      )}
      {mfa.email_available && (
        <Link to="/auth/mfa/email">
          <span>
            {m.login_mfa_use_instead({
              method: m.login_mfa_alternative_email(),
            })}
          </span>
        </Link>
      )}
      <Link to="/auth/mfa/recovery">
        <span>
          {m.login_mfa_use_instead({
            method: m.login_mfa_alternative_recovery(),
          })}
        </span>
      </Link>
      <Link to="/auth/login">
        <span>{m.login_mfa_alternative_back()}</span>
      </Link>
    </div>
  );
};
