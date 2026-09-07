import { useCallback, useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import { AclListTab, type AclListTabValue } from '../../../shared/aclTabs';
import api from '../../../shared/api/api';
import type { LicenseInfo } from '../../../shared/api/types';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../../shared/defguard-ui/components/Menu/types';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import type { OpenConfirmActionModal } from '../../../shared/hooks/modalControls/types';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';

export type AclBulkKind = 'rule' | 'alias' | 'destination';

type MessageFn = () => string;
type CountMessageFn = (args: { count: number }) => string;
type BulkRequest = (ids: number[]) => Promise<unknown>;

type ToggleConfig = {
  title: MessageFn;
  content: CountMessageFn;
  success: MessageFn;
  error: MessageFn;
  noEligible: MessageFn;
  submit: MessageFn;
  request: BulkRequest;
};

type KindConfig = {
  // Use a plural prefix so table selectors do not match row selectors.
  testIdPrefix: string;
  apply: BulkRequest;
  bulkDelete: BulkRequest;
  deployTitle: MessageFn;
  deployContent: CountMessageFn;
  deploySuccess: MessageFn;
  deployError: MessageFn;
  deleteTitle: MessageFn;
  deleteContent: CountMessageFn;
  deleteSuccess: MessageFn;
  deleteError: MessageFn;
  // Aliases and destinations can be referenced by rules; rules obviously cannot.
  deleteNoEligible?: MessageFn;
  // Only rules support enable and disable actions.
  toggles?: { enable: ToggleConfig; disable: ToggleConfig };
};

const config: Record<AclBulkKind, KindConfig> = {
  rule: {
    testIdPrefix: 'rules',
    apply: api.acl.rule.applyRules,
    bulkDelete: api.acl.rule.bulkDeleteRules,
    deployTitle: m.acl_rules_modal_bulk_deploy_title,
    deployContent: m.acl_rules_modal_bulk_deploy_content,
    deploySuccess: m.acl_rules_bulk_deploy_success,
    deployError: m.acl_rules_bulk_deploy_error,
    deleteTitle: m.acl_rules_modal_bulk_delete_title,
    deleteContent: m.acl_rules_modal_bulk_delete_content,
    deleteSuccess: m.acl_rules_bulk_delete_success,
    deleteError: m.acl_rules_bulk_delete_error,
    toggles: {
      enable: {
        title: m.acl_rules_modal_bulk_enable_title,
        content: m.acl_rules_modal_bulk_enable_content,
        success: m.acl_rules_bulk_enable_success,
        error: m.acl_rules_bulk_enable_error,
        noEligible: m.acl_rules_bulk_enable_no_eligible,
        submit: m.controls_enable,
        request: api.acl.rule.bulkEnableRules,
      },
      disable: {
        title: m.acl_rules_modal_bulk_disable_title,
        content: m.acl_rules_modal_bulk_disable_content,
        success: m.acl_rules_bulk_disable_success,
        error: m.acl_rules_bulk_disable_error,
        noEligible: m.acl_rules_bulk_disable_no_eligible,
        submit: m.controls_disable,
        request: api.acl.rule.bulkDisableRules,
      },
    },
  },
  alias: {
    testIdPrefix: 'aliases',
    apply: api.acl.alias.applyAliases,
    bulkDelete: api.acl.alias.bulkDeleteAliases,
    deployTitle: m.acl_aliases_modal_bulk_deploy_title,
    deployContent: m.acl_aliases_modal_bulk_deploy_content,
    deploySuccess: m.acl_aliases_bulk_deploy_success,
    deployError: m.acl_aliases_bulk_deploy_error,
    deleteTitle: m.acl_aliases_modal_bulk_delete_title,
    deleteContent: m.acl_aliases_modal_bulk_delete_content,
    deleteSuccess: m.acl_aliases_bulk_delete_success,
    deleteError: m.acl_aliases_bulk_delete_error,
    deleteNoEligible: m.acl_aliases_bulk_delete_no_eligible,
  },
  destination: {
    testIdPrefix: 'destinations',
    apply: api.acl.destination.applyDestinations,
    bulkDelete: api.acl.destination.bulkDeleteDestinations,
    deployTitle: m.acl_destinations_modal_bulk_deploy_title,
    deployContent: m.acl_destinations_modal_bulk_deploy_content,
    deploySuccess: m.acl_destinations_bulk_deploy_success,
    deployError: m.acl_destinations_bulk_deploy_error,
    deleteTitle: m.acl_destinations_modal_bulk_delete_title,
    deleteContent: m.acl_destinations_modal_bulk_delete_content,
    deleteSuccess: m.acl_destinations_bulk_delete_success,
    deleteError: m.acl_destinations_bulk_delete_error,
    deleteNoEligible: m.acl_destinations_bulk_delete_no_eligible,
  },
};

type BulkTarget = {
  id: number;
  enabled?: boolean;
  rules?: number[];
};

type Props = {
  kind: AclBulkKind;
  selected: BulkTarget[];
  variant: AclListTabValue;
  license: LicenseInfo | null | undefined;
  clearSelection: () => void;
};

export const useAclBulkActions = ({
  kind,
  selected,
  variant,
  license,
  clearSelection,
}: Props): MenuItemsGroup[] => {
  const text = config[kind];

  const confirmBulk = useCallback(
    (data: OpenConfirmActionModal) => {
      // Wait for the license query before showing an upgrade prompt.
      if (license === undefined) return;
      licenseActionCheck(canUseBusinessFeature(license), () => {
        openModal(ModalName.ConfirmAction, {
          ...data,
          invalidateKeys: [['acl']],
          onSuccess: (result) => {
            clearSelection();
            data.onSuccess?.(result);
          },
        });
      });
    },
    [license, clearSelection],
  );

  const handleBulkDeploy = useCallback(() => {
    const ids = selected.map((item) => item.id);
    if (ids.length === 0) return;
    confirmBulk({
      title: text.deployTitle(),
      contentMd: text.deployContent({ count: ids.length }),
      actionPromise: () => text.apply(ids),
      submitProps: { text: m.controls_deploy() },
      onSuccess: () => Snackbar.default(text.deploySuccess()),
      onError: () => Snackbar.error(text.deployError()),
    });
  }, [confirmBulk, selected, text]);

  const handleBulkDelete = useCallback(() => {
    // Exclude referenced items because the backend rolls back the entire batch on failure.
    const ids = selected
      .filter((item) => (item.rules?.length ?? 0) === 0)
      .map((item) => item.id);
    if (ids.length === 0) {
      if (text.deleteNoEligible) Snackbar.warning(text.deleteNoEligible());
      return;
    }
    confirmBulk({
      title: text.deleteTitle(),
      contentMd: text.deleteContent({ count: ids.length }),
      actionPromise: () => text.bulkDelete(ids),
      submitProps: { text: m.controls_delete(), variant: 'critical' },
      onSuccess: () => Snackbar.default(text.deleteSuccess()),
      onError: () => Snackbar.error(text.deleteError()),
    });
  }, [confirmBulk, selected, text]);

  const handleBulkSetEnabled = useCallback(
    (enabled: boolean) => {
      const toggle = enabled ? text.toggles?.enable : text.toggles?.disable;
      if (!toggle) return;
      const ids = selected
        .filter((item) => item.enabled !== enabled)
        .map((item) => item.id);
      if (ids.length === 0) {
        Snackbar.warning(toggle.noEligible());
        return;
      }
      confirmBulk({
        title: toggle.title(),
        contentMd: toggle.content({ count: ids.length }),
        actionPromise: () => toggle.request(ids),
        submitProps: {
          text: toggle.submit(),
          variant: enabled ? undefined : 'critical',
        },
        onSuccess: () => Snackbar.default(toggle.success()),
        onError: () => Snackbar.error(toggle.error()),
      });
    },
    [confirmBulk, selected, text],
  );

  return useMemo((): MenuItemsGroup[] => {
    const prefix = text.testIdPrefix;
    const deleteGroup: MenuItemsGroup = {
      items: [
        {
          text: m.controls_delete(),
          icon: 'delete',
          variant: 'danger',
          testId: `${prefix}-bulk-delete`,
          onClick: handleBulkDelete,
        },
      ],
    };

    if (variant === AclListTab.Pending) {
      return [
        {
          items: [
            {
              text: m.controls_deploy(),
              icon: 'deploy',
              testId: `${prefix}-bulk-deploy`,
              onClick: handleBulkDeploy,
            },
          ],
        },
        deleteGroup,
      ];
    }

    if (text.toggles) {
      const toggleItems: MenuItemProps[] = [
        {
          text: m.controls_enable(),
          icon: 'check',
          testId: `${prefix}-bulk-enable`,
          onClick: () => handleBulkSetEnabled(true),
        },
        {
          text: m.controls_disable(),
          icon: 'disabled',
          testId: `${prefix}-bulk-disable`,
          onClick: () => handleBulkSetEnabled(false),
        },
      ];
      return [{ items: toggleItems }, deleteGroup];
    }

    return [deleteGroup];
  }, [variant, text, handleBulkDeploy, handleBulkDelete, handleBulkSetEnabled]);
};
