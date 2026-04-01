export interface DestinationLabelProps {
  name: string;
  addresses?: string;
  ports?: string;
  protocols?: string;
  anyAddress?: boolean;
  anyPort?: boolean;
  anyProtocol?: boolean;
}
