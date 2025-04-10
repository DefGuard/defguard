import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useCallback } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { EditButton } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';
import { AclAliasStatus } from '../../../../types';
import { useAclAliasDeleteBlockModal } from '../modals/AclAliasDeleteBlockModal/store';
import { useAclAliasCEModal } from '../modals/AlcAliasCEModal/store';
import { AclAliasListData } from '../types';

type EditProps = {
  alias: AclAliasListData;
};

export const AliasEditButton = ({ alias }: EditProps) => {
  const queryClient = useQueryClient();
  const isApplied = alias.state === AclAliasStatus.APPLIED;
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.aliases.list.editMenu;
  const toaster = useToaster();
  const openDeleteBlockModal = useAclAliasDeleteBlockModal((s) => s.open, shallow);

  const {
    acl: {
      aliases: { deleteAlias },
    },
  } = useApi();

  const invalidateQueries = useCallback(() => {
    void queryClient.invalidateQueries({
      predicate: (query) => query.queryKey.includes(QueryKeys.FETCH_ACL_ALIASES),
    });
  }, [queryClient]);

  const handleError = useCallback(
    (err: AxiosError) => {
      toaster.error(LL.acl.listPage.message.changeFail());
      console.error(err.message ?? err);
    },
    [LL.acl.listPage.message, toaster],
  );

  const { mutate: deleteAliasMutation, isPending: deletionPending } = useMutation({
    mutationFn: deleteAlias,
    onSuccess: () => {
      invalidateQueries();
      if (isApplied) {
        toaster.success(LL.acl.listPage.aliases.message.aliasDeleted());
      } else {
        toaster.success(LL.acl.listPage.message.changeDiscarded());
      }
    },
    onError: handleError,
  });

  const openEditModal = useAclAliasCEModal((s) => s.open, shallow);

  return (
    <EditButton disabled={deletionPending}>
      <EditButtonOption
        text={LL.common.controls.edit()}
        onClick={() => {
          openEditModal({ alias });
        }}
        disabled={deletionPending}
      />
      <EditButtonOption
        disabled={deletionPending}
        text={isApplied ? localLL.delete() : localLL.discardChanges()}
        styleVariant={EditButtonOptionStyleVariant.WARNING}
        onClick={() => {
          if (alias.rules.length === 0) {
            deleteAliasMutation(alias.id);
          } else {
            openDeleteBlockModal(alias);
          }
        }}
      />
    </EditButton>
  );
};
