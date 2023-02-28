export type User = {
  userName: string;
  email: string;
  locations: Location[];
};

export interface Location {
  name: string;
  ipAddress: string;
  shared: {
    ipAddress: string;
  }[];
}

export enum NetworkTypeEnum {
  MESH = 'mesh',
  REGULAR = 'regular',
}

export type WizardNetwork = {
  type?: 'mesh' | 'regular';
  name?: string;
  address?: string;
  port?: number;
  endpoint?: string;
  allowedIps?: string;
  dns?: string;
  id?: string;
};

export type FormStatus = { [key: number]: boolean };
