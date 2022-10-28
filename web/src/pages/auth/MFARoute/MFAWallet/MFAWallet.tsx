import { useMutation } from '@tanstack/react-query';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import useApi from '../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../shared/mutations';

export const MFAWallet = () => {
  const {
    auth: {
      mfa: {
        web3: { start },
      },
    },
  } = useApi();
  const { mutate, isLoading } = useMutation(
    [MutationKeys.WEB3_MFA_START],
    start,
    {
      onSuccess: (data) => {
        console.log(data);
      },
    }
  );

  return (
    <>
      <p>
        Use your crypto wallet to sign in, please sign message in your wallet
        app or extension.
      </p>
      <Button
        text="Use your wallet"
        styleVariant={ButtonStyleVariant.PRIMARY}
        size={ButtonSize.BIG}
        loading={isLoading}
        onClick={() => mutate()}
      />
      <div className="mfa-methods"></div>
      <nav>
        <span>or</span>
        <Button text="Use authenticator app instead" />
        <Button text="Use security key insted" />
      </nav>
    </>
  );
};
