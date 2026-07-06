import { useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import type { LdapDryRunResult } from '../../../../../shared/api/types';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { Tabs } from '../../../../../shared/defguard-ui/components/Tabs/Tabs';
import './style.scss';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { DryRunTable } from './DryRunTable';

type Props = {
  isOpen: boolean;
  result: LdapDryRunResult | null;
  onClose: () => void;
};

type DryRunTab = 'defguard' | 'ldap';

export const LdapDryRunModal = ({ isOpen, result, onClose }: Props) => {
  const [tab, setTab] = useState<DryRunTab>('defguard');

  const activeData = (tab === 'defguard' ? result?.defguard : result?.ldap) ?? [];

  return (
    <Modal
      title={m.modal_ldap_dry_run_title()}
      size="primary"
      isOpen={isOpen}
      onClose={onClose}
      contentClassName="ldap-dry-run-modal"
    >
      <p className="description">{m.modal_ldap_dry_run_description()}</p>
      <SizedBox height={ThemeSpacing.Xl} />
      <Tabs
        items={[
          {
            title: m.modal_ldap_dry_run_tab_defguard(),
            active: tab === 'defguard',
            onClick: () => setTab('defguard'),
          },
          {
            title: m.modal_ldap_dry_run_tab_ldap(),
            active: tab === 'ldap',
            onClick: () => setTab('ldap'),
          },
        ]}
      />
      <DryRunTable key={tab} data={activeData} />
    </Modal>
  );
};
