import './style.scss';

type Props = {
  config: string;
  title?: string;
  keys: {
    publicKey: string;
    privateKey?: string;
  };
};

export const StandaloneDeviceModalConfigExpandable = ({ config, keys, title }: Props) => {
  return <></>;
};
