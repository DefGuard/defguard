import type { PropsWithChildren } from 'react';
import { m } from '../../../../paraglide/messages';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../../../shared/defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';

export const ProviderSyncToggle = ({ children }: PropsWithChildren) => {
  const enabled = useAddExternalOpenIdStore(
    (s) => s.providerState.directory_sync_enabled,
  );

  return (
    <>
      <Toggle
        active={enabled}
        onClick={() => {
          useAddExternalOpenIdStore.setState((s) => ({
            providerState: { ...s.providerState, directory_sync_enabled: !enabled },
          }));
        }}
        label={m.settings_openid_provider_directory_sync_toggle()}
      />
      <Fold open={enabled}>
        <SizedBox height={ThemeSpacing.Xl} />
        {children}
      </Fold>
      <SizedBox height={ThemeSpacing.Xl} />
      <Divider />
    </>
  );
};
