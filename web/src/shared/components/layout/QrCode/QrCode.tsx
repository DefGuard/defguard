import { QRCode } from 'react-qrcode';

interface Props {
  data: string;
}
export const QrCode = ({ data }: Props) => {
  return <QRCode value={data} className="qr-code" width={300} />;
};
