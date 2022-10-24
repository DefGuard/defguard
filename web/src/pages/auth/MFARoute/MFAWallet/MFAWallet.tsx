import Button from '../../../../shared/components/layout/Button/Button';
import { CheckBox } from '../../../../shared/components/layout/Checkbox/CheckBox';

export const MFAWallet = () => {
  return (
    <>
      <p>
        Use your crypto wallet to sign in, please sign message in your wallet
        app or extension.
      </p>
      <Button />
      <label>
        <CheckBox value={0} />
        Use this method for future logins
      </label>
      <p>or</p>
      <div className="mfa-methods">
        <Button text="Use authenticator app instead" />
        <Button text="Use security key insted" />
      </div>
    </>
  );
};
