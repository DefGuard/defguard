import { useQRCode } from 'next-qrcode';

interface Props {
  data: string;
}
export const QrCode = ({ data }: Props) => {
  const { Canvas } = useQRCode();
  return (
    <Canvas
      text={data}
      options={{
        width: 300,
      }}
    />
  );
};
