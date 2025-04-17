export enum AddStandaloneDeviceModalStep {
  METHOD_CHOICE,
  SETUP_CLI,
  FINISH_CLI,
  SETUP_MANUAL,
  FINISH_MANUAL,
}

export enum AddStandaloneDeviceModalChoice {
  CLI,
  MANUAL,
}

export enum WGConfigGenChoice {
  MANUAL,
  AUTO,
}

export type AddStandaloneDeviceFormFields = {
  name: string;
  location_id: number;
  modifiableIpParts: string[];
  wireguard_pubkey?: string;
  generationChoice: WGConfigGenChoice;
  description?: string;
};

export type AddStandaloneDeviceCLIFormFields = Omit<
  AddStandaloneDeviceFormFields,
  'generationChoice' | 'wireguard_pubkey'
>;
