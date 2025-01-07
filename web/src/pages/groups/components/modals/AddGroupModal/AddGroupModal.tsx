import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Divider } from '../../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { Search } from '../../../../../shared/defguard-ui/components/Layout/Search/Search';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { ModifyGroupsRequest } from '../../../../../shared/types';
import { invalidateMultipleQueries } from '../../../../../shared/utils/invalidateMultipleQueries';
import { GroupFormSelectAll } from './components/GroupFormSelectAll/GroupFormSelectAll';
import { UserSelect } from './components/UserSelect/UserSelect';
import { useAddGroupModal } from './useAddGroupModal';

export const AddGroupModal = () => {
  const isOpen = useAddGroupModal((s) => s.visible);
  const close = useAddGroupModal((s) => s.close);
  const groupInfo = useAddGroupModal((s) => s.groupInfo);
  const { LL } = useI18nContext();

  return (
    <ModalWithTitle
      id="modify-group-modal"
      title={groupInfo ? LL.modals.editGroup.title() : LL.modals.addGroup.title()}
      isOpen={isOpen}
      onClose={close}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const toInvalidate = [QueryKeys.FETCH_GROUPS, QueryKeys.FETCH_GROUPS_INFO];

export type ModifyGroupFormFields = {
  name: string;
  members: string[];
  is_admin: boolean;
};

const ModalContent = () => {
  const {
    groups: { getGroups, createGroup, editGroup },
    user: { getUsers },
  } = useApi();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const groupInfo = useAddGroupModal((s) => s.groupInfo);
  const localLL = groupInfo ? LL.modals.editGroup : LL.modals.addGroup;
  const closeModal = useAddGroupModal((s) => s.close);
  const toaster = useToaster();
  const [searchValue, setSearch] = useState('');

  const { data: groups } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS],
    queryFn: async () => getGroups().then((d) => d.groups),
  });

  const { data: users } = useQuery({
    queryKey: [QueryKeys.FETCH_USERS_LIST],
    queryFn: getUsers,
  });

  const { mutate: createGroupMutation, isPending: isCreating } = useMutation({
    mutationFn: createGroup,
    onSuccess: () => {
      toaster.success(LL.messages.success());
      invalidateMultipleQueries(queryClient, toInvalidate);
      closeModal();
    },
  });

  const { mutate: editGroupMutation, isPending: isEditing } = useMutation({
    mutationFn: editGroup,
    onSuccess: () => {
      toaster.success(LL.messages.success());
      invalidateMultipleQueries(queryClient, toInvalidate);
      closeModal();
    },
  });

  const filteredUsers = useMemo(() => {
    if (users) {
      const loweredSearch = searchValue.toLocaleLowerCase();
      return users.filter(
        (u) =>
          u.username.toLocaleLowerCase().includes(loweredSearch) ||
          u.first_name.toLowerCase().includes(loweredSearch) ||
          u.last_name.toLowerCase().includes(loweredSearch),
      );
    }
    return [];
  }, [searchValue, users]);

  const schema = useMemo(
    () =>
      z.object({
        name: z
          .string({
            required_error: LL.form.error.required(),
          })
          .min(4, LL.form.error.minimumLength())
          .refine((name) => {
            // if in edit mode ignore self name
            let names = groups;
            if (!isUndefined(groupInfo)) {
              names = names?.filter((n) => n !== groupInfo.name);
            }
            return isUndefined(names?.find((n) => n === name));
          }, LL.form.error.invalid()),
        members: z.array(z.string()),
        is_admin: z.boolean(),
      }),
    [LL.form.error, groupInfo, groups],
  );

  const defaults = useMemo((): ModifyGroupFormFields => {
    if (groupInfo) {
      return {
        name: groupInfo.name,
        members: groupInfo.members ?? [],
        is_admin: groupInfo.is_admin,
      };
    }
    return {
      name: '',
      members: [],
      is_admin: false,
    };
  }, [groupInfo]);

  const {
    handleSubmit,
    control,
    formState: { isValidating, isSubmitting },
  } = useForm<ModifyGroupFormFields>({
    defaultValues: defaults,
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<ModifyGroupFormFields> = (values) => {
    const sendValues: ModifyGroupsRequest = {
      name: values.name,
      members: values.members,
      is_admin: values.is_admin,
    };
    if (groupInfo) {
      editGroupMutation({ ...sendValues, originalName: groupInfo.name });
    } else {
      createGroupMutation(sendValues);
    }
  };

  return (
    <form onSubmit={void handleSubmit(handleValidSubmit)}>
      <FormInput controller={{ control, name: 'name' }} label={localLL.groupName()} />
      <div className="group-settings">
        <label>{localLL.groupSettings()}:</label>
        <FormCheckBox
          controller={{ control, name: 'is_admin' }}
          label={localLL.adminGroup()}
          labelPlacement="right"
        />
      </div>
      <Divider />
      {users && <GroupFormSelectAll users={users} control={control} />}
      <Divider />
      <div className="search-wrapper">
        <Search
          placeholder={localLL.searchPlaceholder()}
          debounceTiming={500}
          onDebounce={(val) => setSearch(val ?? '')}
        />
      </div>
      <div className="users">
        <div className="scroll-wrapper">
          {filteredUsers.map((user) => (
            <UserSelect user={user} key={user.id} control={control} />
          ))}
        </div>
      </div>
      <div className="controls">
        <Button
          size={ButtonSize.LARGE}
          onClick={() => closeModal()}
          text={LL.common.controls.cancel()}
          type="button"
        />
        <Button
          size={ButtonSize.LARGE}
          disabled={isUndefined(groups)}
          loading={isCreating || isEditing || isValidating || isSubmitting}
          text={localLL.submit()}
          styleVariant={ButtonStyleVariant.PRIMARY}
          type="submit"
        />
      </div>
    </form>
  );
};
