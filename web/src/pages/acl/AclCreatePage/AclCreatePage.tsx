import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { intersection } from 'lodash-es';
import { useCallback, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import { useSearchParams } from 'react-router-dom';
import { z } from 'zod';

import { useI18nContext } from '../../../i18n/i18n-react';
import { PageContainer } from '../../../shared/components/Layout/PageContainer/PageContainer';
import { RenderMarkdown } from '../../../shared/components/Layout/RenderMarkdown/RenderMarkdown';
import { SectionWithCard } from '../../../shared/components/Layout/SectionWithCard/SectionWithCard';
import { FormCheckBox } from '../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { FormTextarea } from '../../../shared/defguard-ui/components/Form/FormTextarea/FormTextarea';
import { ActivityIcon } from '../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { LabeledCheckbox } from '../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { MessageBox } from '../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import {
  MessageBoxStyleVariant,
  MessageBoxType,
} from '../../../shared/defguard-ui/components/Layout/MessageBox/types';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';
import {
  AclRuleInfo,
  CreateAclRuleRequest,
  EditAclRuleRequest,
  GroupInfo,
  Network,
  StandaloneDevice,
  User,
} from '../../../shared/types';
import { trimObjectStrings } from '../../../shared/utils/trimObjectStrings';
import { useAclLoadedContext } from '../acl-context';
import { protocolOptions, protocolToString } from '../utils';
import { aclPortsValidator } from '../validators';
import { FormDialogSelect } from './components/DialogSelect/FormDialogSelect';

// type Alias = {
//   id: number;
//   name: string;
// };

type AclForm = Omit<AclRuleInfo, 'parent_id' | 'state'>;

export const AlcCreatePage = () => {
  const [searchParams] = useSearchParams();
  const editMode = ['1', 'true'].includes(searchParams.get('edit') ?? '');
  const { LL } = useI18nContext();
  const localLL = LL.acl.createPage;
  const labelsLL = localLL.labels;
  const formErrors = LL.form.error;
  const { networks, devices, groups, users, ruleToEdit } = useAclLoadedContext();
  const queryClient = useQueryClient();

  const initialValue = useMemo(() => {
    if (editMode) {
      return ruleToEdit as AclForm;
    }
    const defaultValue: AclForm = {
      aliases: [],
      all_networks: false,
      allow_all_users: false,
      allow_all_network_devices: false,
      allowed_devices: [],
      allowed_groups: [],
      allowed_users: [],
      denied_devices: [],
      denied_groups: [],
      denied_users: [],
      deny_all_users: false,
      deny_all_network_devices: false,
      destination: '',
      id: 0,
      name: '',
      networks: [],
      ports: '',
      protocols: [],
      expires: undefined,
      enabled: true,
    };
    return defaultValue;
  }, [editMode, ruleToEdit]);

  // const [neverExpires, setNeverExpires] = useState(!isPresent(initialValue.expires));
  const [allowAllUsers, setAllowAllUsers] = useState(initialValue.allow_all_users);
  const [denyAllUsers, setDenyAllUsers] = useState(initialValue.deny_all_users);
  const [allowAllNetworkDevices, setAllowAllNetworkDevices] = useState(
    initialValue.allow_all_network_devices,
  );
  const [denyAllNetworkDevices, setDenyAllNetworkDevices] = useState(
    initialValue.deny_all_network_devices,
  );
  const [allowAllLocations, setAllowAllLocations] = useState(initialValue.all_networks);
  const submitRef = useRef<HTMLInputElement | null>(null);

  const navigate = useNavigate();

  const {
    acl: {
      rules: { createRule, editRule },
    },
  } = useApi();

  const handleSuccess = useCallback(() => {
    const keys = [QueryKeys.FETCH_ACL_RULES, QueryKeys.FETCH_ACL_RULE_EDIT];
    for (const key of keys) {
      void queryClient.invalidateQueries({
        predicate: (query) => query.queryKey.includes(key),
      });
    }
    navigate('/admin/acl');
  }, [navigate, queryClient]);

  const { mutate: mutatePost, isPending: postPending } = useMutation({
    mutationFn: createRule,
    onSuccess: () => {
      handleSuccess();
    },
  });

  const { mutate: mutatePut, isPending: putPending } = useMutation({
    mutationFn: editRule,
    onSuccess: () => {
      handleSuccess();
    },
  });

  const schema = useMemo(
    () =>
      z
        .object({
          name: z
            .string({
              required_error: formErrors.required(),
            })
            .min(1, formErrors.required()),
          networks: z.number().array(),
          expires: z.string().nullable(),
          enabled: z.boolean(),
          allowed_users: z.number().array(),
          denied_users: z.number().array(),
          allowed_groups: z.number().array(),
          denied_groups: z.number().array(),
          allowed_devices: z.number().array(),
          denied_devices: z.number().array(),
          aliases: z.number().array(),
          destination: z.string(),
          ports: aclPortsValidator(LL),
          protocols: z.number().array(),
        })
        .superRefine((vals, ctx) => {
          // check for collisions
          const message = LL.acl.createPage.formError.allowDenyConflict();
          if (!allowAllUsers && !denyAllUsers) {
            if (intersection(vals.allowed_users, vals.denied_users).length) {
              ctx.addIssue({
                path: ['allowed_users'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_users'],
                code: 'custom',
                message,
              });
            }
            if (intersection(vals.allowed_groups, vals.denied_groups).length) {
              ctx.addIssue({
                path: ['allowed_groups'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_groups'],
                code: 'custom',
                message,
              });
            }
          }
          if (!allowAllNetworkDevices && !denyAllNetworkDevices) {
            if (intersection(vals.allowed_devices, vals.denied_devices).length) {
              ctx.addIssue({
                path: ['allowed_devices'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_devices'],
                code: 'custom',
                message,
              });
            }
          }
        }),
    [
      LL,
      allowAllNetworkDevices,
      allowAllUsers,
      denyAllNetworkDevices,
      denyAllUsers,
      formErrors,
    ],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo((): FormFields => {
    const res: FormFields = {
      aliases: initialValue.aliases,
      allowed_devices: initialValue.allowed_devices,
      allowed_groups: initialValue.allowed_groups,
      allowed_users: initialValue.allowed_users,
      denied_devices: initialValue.denied_devices,
      denied_groups: initialValue.denied_groups,
      denied_users: initialValue.denied_users,
      destination: initialValue.destination,
      expires: initialValue.expires ?? null,
      name: initialValue.name,
      networks: initialValue.networks,
      ports: initialValue.ports,
      protocols: initialValue.protocols,
      enabled: initialValue.enabled,
    };
    return res;
  }, [initialValue]);

  const { control, handleSubmit, trigger } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(schema),
    criteriaMode: 'all',
  });

  // const watchedExpires = watch('expires');

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    const cleaned = trimObjectStrings(values);
    let expires = cleaned.expires;
    // todo: remove this when DateInput will have time implemented, for now expires date means 00:00 of the day selected
    if (expires) {
      expires = dayjs(expires).utc().startOf('day').toISOString();
    }

    if (editMode) {
      const requestData: EditAclRuleRequest = {
        ...cleaned,
        allow_all_users: allowAllUsers,
        deny_all_users: denyAllUsers,
        allow_all_network_devices: allowAllNetworkDevices,
        deny_all_network_devices: denyAllNetworkDevices,
        all_networks: allowAllLocations,
        id: initialValue.id,
        expires,
      };
      mutatePut(requestData);
    } else {
      const requestData: CreateAclRuleRequest = {
        ...cleaned,
        allow_all_users: allowAllUsers,
        deny_all_users: denyAllUsers,
        allow_all_network_devices: allowAllNetworkDevices,
        deny_all_network_devices: denyAllNetworkDevices,
        all_networks: allowAllLocations,
        expires,
      };
      mutatePost(requestData);
    }
  };

  return (
    <PageContainer id="acl-create-page">
      <div className="header">
        <h1>{LL.acl.sharedTitle()}</h1>
        <div className="controls">
          <Button
            text={LL.common.controls.cancel()}
            onClick={() => {
              navigate('/admin/acl');
            }}
            disabled={postPending || putPending}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
          />
          <Button
            type="submit"
            text={LL.common.controls.submit()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            loading={postPending || putPending}
            onClick={() => {
              submitRef.current?.click();
            }}
          />
        </div>
      </div>
      <form id="acl-sections" onSubmit={handleSubmit(handleValidSubmit)}>
        <SectionWithCard title={localLL.headers.rule()} id="rule-card">
          <FormInput controller={{ control, name: 'name' }} label="Rule Name" />
          <LabeledCheckbox
            label={labelsLL.allowAllNetworks()}
            value={allowAllLocations}
            onChange={setAllowAllLocations}
          />
          <FormCheckBox
            controller={{ control, name: 'enabled' }}
            label={LL.common.controls.enabled()}
            labelPlacement="right"
          />
          <FormDialogSelect
            controller={{ control, name: 'networks' }}
            options={networks}
            renderTagContent={renderNetworkSelectTag}
            identKey="id"
            label={labelsLL.locations()}
            searchKeys={['name']}
            disabled={allowAllLocations}
          />
          {/* <CardHeader title="Expiration Date" />
          <LabeledCheckbox
            label="Never Expire"
            value={neverExpires && watchedExpires === null}
            onChange={(change) => {
              if (change) {
                setValue('expires', null, {
                  shouldValidate: false,
                  shouldDirty: true,
                });
              }
              setNeverExpires(change);
            }}
          />
          <FormDateInput
            controller={{ control, name: 'expires' }}
            label="Expiration Date"
            disabled={neverExpires}
          /> */}
        </SectionWithCard>
        <SectionWithCard title={localLL.headers.allowed()} id="allow-card">
          <MessageBox styleVariant={MessageBoxStyleVariant.OUTLINED}>
            <RenderMarkdown content={localLL.infoBox.allowInstructions()} />
          </MessageBox>
          <LabeledCheckbox
            value={allowAllUsers}
            onChange={(val) => {
              if (val) {
                setDenyAllUsers(false);
              }
              setAllowAllUsers(val);
              void trigger('denied_users', { shouldFocus: false });
              void trigger('allowed_users', { shouldFocus: false });
              void trigger('denied_groups', { shouldFocus: false });
              void trigger('allowed_groups', { shouldFocus: false });
            }}
            label={labelsLL.allowAllUsers()}
          />
          <FormDialogSelect
            label={labelsLL.users()}
            controller={{ control, name: 'allowed_users' }}
            options={users}
            renderTagContent={renderUserTag}
            renderDialogListItem={renderUserListItem}
            identKey="id"
            searchKeys={['email', 'last_name', 'first_name']}
            disabled={allowAllUsers}
            onChange={() => {
              void trigger('denied_users', { shouldFocus: false });
            }}
          />
          <FormDialogSelect
            label={labelsLL.groups()}
            controller={{ control, name: 'allowed_groups' }}
            options={groups}
            renderTagContent={renderGroup}
            identKey="id"
            searchKeys={['name']}
            disabled={allowAllUsers}
            onChange={() => {
              void trigger('denied_groups', {
                shouldFocus: false,
              });
            }}
          />
          <LabeledCheckbox
            value={allowAllNetworkDevices}
            onChange={(val) => {
              if (val) {
                setDenyAllNetworkDevices(false);
              }
              setAllowAllNetworkDevices(val);
              void trigger('denied_devices', { shouldFocus: false });
              void trigger('allowed_devices', { shouldFocus: false });
            }}
            label={labelsLL.allowAllNetworkDevices()}
          />
          <FormDialogSelect
            label={labelsLL.devices()}
            controller={{ control, name: 'allowed_devices' }}
            options={devices}
            renderTagContent={renderNetworkDevice}
            identKey="id"
            searchKeys={['name']}
            disabled={allowAllNetworkDevices}
            onChange={() => {
              void trigger('denied_devices', {
                shouldFocus: false,
              });
            }}
          />
        </SectionWithCard>
        <SectionWithCard title={localLL.headers.destination()} id="destination-card">
          <MessageBox
            styleVariant={MessageBoxStyleVariant.OUTLINED}
            type={MessageBoxType.INFO}
          >
            <RenderMarkdown content={localLL.infoBox.destinationInstructions()} />
          </MessageBox>
          {/* <FormDialogSelect
            controller={{ control, name: 'aliases' }}
            options={aliases}
            label="Aliases"
            identKey="id"
            renderTagContent={renderAlias}
            searchKeys={['name']}
          /> */}
          {/* <CardHeader title="Manual Input" /> */}
          <FormTextarea
            controller={{ control, name: 'destination' }}
            label={labelsLL.manualIp()}
          />
          <FormInput controller={{ control, name: 'ports' }} label={labelsLL.ports()} />
          <FormSelect
            controller={{ control, name: 'protocols' }}
            label={labelsLL.protocols()}
            placeholder={localLL.placeholders.allProtocols()}
            options={protocolOptions}
            searchable={false}
            renderSelected={(val) => ({ displayValue: protocolToString(val), key: val })}
            disposable
          />
        </SectionWithCard>
        <SectionWithCard title={localLL.headers.denied()} id="denied-card">
          <MessageBox styleVariant={MessageBoxStyleVariant.OUTLINED}>
            <RenderMarkdown content={localLL.infoBox.allowInstructions()} />
          </MessageBox>
          <LabeledCheckbox
            label={labelsLL.denyAllUsers()}
            value={denyAllUsers}
            onChange={(val) => {
              if (val) {
                setAllowAllUsers(false);
              }
              setDenyAllUsers(val);
              void trigger('denied_users', { shouldFocus: false });
              void trigger('allowed_users', { shouldFocus: false });
              void trigger('denied_groups', { shouldFocus: false });
              void trigger('allowed_groups', { shouldFocus: false });
            }}
          />
          <FormDialogSelect
            label={labelsLL.users()}
            controller={{ control, name: 'denied_users' }}
            options={users}
            renderTagContent={renderUserTag}
            renderDialogListItem={renderUserListItem}
            identKey="id"
            searchKeys={['username', 'first_name', 'last_name']}
            disabled={denyAllUsers}
            onChange={() => {
              void trigger('allowed_users', {
                shouldFocus: false,
              });
            }}
          />
          <FormDialogSelect
            label={labelsLL.groups()}
            controller={{ control, name: 'denied_groups' }}
            options={groups}
            renderTagContent={renderGroup}
            identKey="id"
            searchKeys={['name']}
            disabled={denyAllUsers}
            onChange={() => {
              void trigger('allowed_groups', {
                shouldFocus: false,
              });
            }}
          />
          <LabeledCheckbox
            label={labelsLL.denyAllNetworkDevices()}
            value={denyAllNetworkDevices}
            onChange={(val) => {
              if (val) {
                setAllowAllNetworkDevices(false);
              }
              setDenyAllNetworkDevices(val);
              void trigger('denied_devices', { shouldFocus: false });
              void trigger('allowed_devices', { shouldFocus: false });
            }}
          />
          <FormDialogSelect
            label={labelsLL.devices()}
            controller={{ control, name: 'denied_devices' }}
            options={devices}
            renderTagContent={renderNetworkDevice}
            identKey="id"
            searchKeys={['name']}
            disabled={denyAllNetworkDevices}
            onChange={() => {
              void trigger('allowed_devices', {
                shouldFocus: false,
              });
            }}
          />
        </SectionWithCard>
        <input type="submit" ref={submitRef} className="hidden" />
      </form>
    </PageContainer>
  );
};

// const CardHeader = ({ title }: { title: string }) => {
//   return (
//     <div className="header">
//       <h3>{title}</h3>
//       <hr />
//     </div>
//   );
// };

const renderNetworkSelectTag = (network: Network) => (
  <>
    <p>{network.name}</p>
    <ActivityIcon
      status={
        network.acl_default_allow
          ? ActivityIconVariant.CONNECTED
          : ActivityIconVariant.ERROR_FILLED
      }
    />
  </>
);

const renderUserTag = (user: User) => <p>{user.username}</p>;

const renderUserListItem = (user: User) => (
  <>
    <p>{`${user.first_name} ${user.last_name} (${user.username})`}</p>
  </>
);

const renderNetworkDevice = (device: StandaloneDevice) => <p>{device.name}</p>;

// const renderAlias = (alias: Alias) => <p>{alias.name}</p>;

const renderGroup = (group: GroupInfo) => <p>{group.name}</p>;
