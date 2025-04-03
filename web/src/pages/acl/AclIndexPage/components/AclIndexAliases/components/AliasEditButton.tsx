import { useCallback } from "react";
import { AclAliasInfo } from "../../../../../../shared/types";
import { ListData } from "../AclIndexAliases";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { AclAliasStatus } from "../../../../types";
import { useI18nContext } from "../../../../../../i18n/i18n-react";
import { useToaster } from "../../../../../../shared/hooks/useToaster";
import useApi from "../../../../../../shared/hooks/useApi";
import { QueryKeys } from "../../../../../../shared/queries";
import { AxiosError } from "axios";
import { EditButton } from "../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton";
import { EditButtonOption } from "../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption";
import { EditButtonOptionStyleVariant } from "../../../../../../shared/defguard-ui/components/Layout/EditButton/types";
import { useAclAliasCEModal } from "../modals/AlcAliasCEModal/store";
import { shallow } from "zustand/shallow";

type EditProps = {
  alias: ListData;
};

export const AliasEditButton = ({ alias }: EditProps) => {
  const queryClient = useQueryClient();
  const isApplied = alias.state === AclAliasStatus.APPLIED;
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.rules.list.editMenu;
  const toaster = useToaster();

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
        toaster.success(LL.acl.listPage.message.changeAdded());
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
        text={isApplied ? localLL.delete() : localLL.discard()}
        styleVariant={EditButtonOptionStyleVariant.WARNING}
        onClick={() => {
          deleteAliasMutation(alias.id);
        }}
      />
    </EditButton>
  );
};
