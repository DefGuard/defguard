import type { ComponentType, ReactNode } from 'react';

export type CERuleFormValues = {
  name: string;
  locations: number[];
  expires: string | null;
  enabled: boolean;
  all_locations: boolean;
  allow_all_users: boolean;
  deny_all_users: boolean;
  allow_all_groups: boolean;
  deny_all_groups: boolean;
  allow_all_network_devices: boolean;
  deny_all_network_devices: boolean;
  allowed_users: number[];
  denied_users: number[];
  allowed_groups: number[];
  denied_groups: number[];
  allowed_network_devices: number[];
  denied_network_devices: number[];
  addresses: string;
  ports: string;
  protocols: Set<number>;
  any_address: boolean;
  any_port: boolean;
  any_protocol: boolean;
  destinations: Set<number>;
  aliases: Set<number>;
  use_manual_destination_settings: boolean;
};

export type CERuleFormApi = {
  AppField: ComponentType<{
    name: keyof CERuleFormValues;
    children: (field: unknown) => ReactNode;
  }>;
  Subscribe: ComponentType<{
    selector: (state: unknown) => unknown;
    children: (value: unknown) => ReactNode;
  }>;
  setFieldValue: (field: keyof CERuleFormValues, value: unknown) => void;
};

export type FormFieldRenderer = Record<string, ComponentType<Record<string, unknown>>>;
