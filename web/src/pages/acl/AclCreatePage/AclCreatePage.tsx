import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import { useSearchParams } from 'react-router-dom';
import { z } from 'zod';

import { useI18nContext } from '../../../i18n/i18n-react';
import { PageContainer } from '../../../shared/components/Layout/PageContainer/PageContainer';
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
import { SelectOption } from '../../../shared/defguard-ui/components/Layout/Select/types';
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
import { AclProtocol } from '../types';
import { FormDialogSelect } from './components/DialogSelect/FormDialogSelect';

type Alias = {
  id: number;
  name: string;
};

type AclForm = Omit<AclRuleInfo, 'parent_id' | 'state'>;

const mockedAliases: Alias[] = [];

export const AlcCreatePage = () => {
  const [searchParams] = useSearchParams();
  const editMode = ['1', 'true'].includes(searchParams.get('edit') ?? '');
  const { LL } = useI18nContext();
  const localLL = LL.acl.createPage;
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
      allowed_devices: [],
      allowed_groups: [],
      allowed_users: [],
      denied_devices: [],
      denied_groups: [],
      denied_users: [],
      deny_all_users: false,
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
      void queryClient.refetchQueries({
        queryKey: [key],
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
      z.object({
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
        ports: z
          .string()
          .refine((value: string) => {
            if (value === '') return true;
            const regexp = new RegExp(
              /^(?:\d+(?:-\d+)*)(?:(?:\s*,\s*|\s+)\d+(?:-\d+)*)*$/,
            );
            return regexp.test(value);
          })
          .refine((value: string) => {
            if (value === '') return true;
            // check if there is no duplicates in given port field
            const trimmed = value
              .replaceAll(' ', '')
              .replaceAll('-', ' ')
              .replaceAll(',', ' ')
              .split(' ')
              .filter((v) => v !== '');
            const found: number[] = [];
            for (const entry of trimmed) {
              const num = parseInt(entry);
              if (isNaN(num)) {
                return false;
              }
              if (found.includes(num)) {
                return false;
              }
              found.push(num);
            }
            return true;
          }, formErrors.invalid())
          .refine((value: string) => {
            if (value === '') return true;
            // check if ranges in input are valid means follow pattern <start>-<end>
            const matches = value.match(/\b\d+-\d+\b/g);
            if (Array.isArray(matches)) {
              for (const match of matches) {
                const split = match.split('-');
                if (split.length !== 2) {
                  return false;
                }
                const start = split[0];
                const end = split[1];
                if (start >= end) {
                  return false;
                }
              }
            }
            return true;
          }, formErrors.invalid()),
        protocols: z.number().array(),
      }),
    [formErrors],
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

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { control, handleSubmit, watch, setValue } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(schema),
  });

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const watchedExpires = watch('expires');

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
        all_networks: allowAllLocations,
        expires,
      };
      mutatePost(requestData);
    }
  };

  return (
    <PageContainer id="acl-create-page">
      <div className="header">
        <h1>{LL.acl.createPage.title()}</h1>
        <div className="controls">
          <Button
            text="Cancel"
            onClick={() => {
              navigate('/admin/acl');
            }}
            disabled={postPending || putPending}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
          />
          <Button
            type="submit"
            text="Submit"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            loading={postPending || putPending}
            onClick={() => {
              submitRef.current?.click();
            }}
          />
        </div>
      </div>
      <form
        id="acl-sections"
        onSubmit={handleSubmit(handleValidSubmit, (vals) => console.log(vals))}
      >
        <SectionWithCard title={localLL.sections.rule.title()} id="rule-card">
          <FormInput controller={{ control, name: 'name' }} label="Rule Name" />
          <LabeledCheckbox
            label="Allow all locations"
            value={allowAllLocations}
            onChange={setAllowAllLocations}
          />
          <FormCheckBox
            controller={{ control, name: 'enabled' }}
            label="Enabled"
            labelPlacement="right"
          />
          <FormDialogSelect
            controller={{ control, name: 'networks' }}
            options={networks}
            renderTagContent={renderNetworkSelectTag}
            identKey="id"
            label="Locations"
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
        <SectionWithCard title="Allowed Users/Devices/Groups" id="allow-card">
          <MessageBox styleVariant={MessageBoxStyleVariant.OUTLINED}>
            <p>
              Specify one or more fields (Users or Groups) to define this rule. The rule
              will consider all inputs provided for matching conditions. Leave any fields
              blank if not needed.
            </p>
          </MessageBox>
          <LabeledCheckbox
            value={allowAllUsers}
            onChange={(val) => {
              if (val) {
                setDenyAllUsers(false);
              }
              setAllowAllUsers(val);
            }}
            label="Allow all users"
          />
          <FormDialogSelect
            label="Users"
            controller={{ control, name: 'allowed_users' }}
            options={users}
            renderTagContent={renderUserTag}
            renderDialogListItem={renderUserListItem}
            identKey="id"
            searchKeys={['email', 'last_name', 'first_name']}
            disabled={allowAllUsers}
          />
          <FormDialogSelect
            label="Groups"
            controller={{ control, name: 'allowed_groups' }}
            options={groups}
            renderTagContent={renderGroup}
            identKey="id"
            searchKeys={['name']}
            disabled={allowAllUsers || true}
          />
          <FormDialogSelect
            label="Network Devices"
            controller={{ control, name: 'allowed_devices' }}
            options={devices}
            renderTagContent={renderNetworkDevice}
            identKey="id"
            searchKeys={['name']}
          />
        </SectionWithCard>
        <SectionWithCard title="Destination" id="destination-card">
          <MessageBox
            styleVariant={MessageBoxStyleVariant.OUTLINED}
            type={MessageBoxType.INFO}
          >
            <p>
              Specify one or more fields (Aliases, IPs, or Ports) to define this rule. The
              rule will consider all inputs provided for matching conditions. Leave any
              fields blank if not needed.
            </p>
          </MessageBox>
          <FormDialogSelect
            controller={{ control, name: 'aliases' }}
            options={mockedAliases}
            label="Aliases"
            identKey="id"
            renderTagContent={renderAlias}
            searchKeys={['name']}
          />
          <CardHeader title="Manual Input" />
          <FormTextarea
            controller={{ control, name: 'destination' }}
            label="IPv4/6 CIDR range or address"
          />
          <FormInput controller={{ control, name: 'ports' }} label="Ports" />
          <FormSelect
            controller={{ control, name: 'protocols' }}
            label="Protocols"
            placeholder="All protocols"
            options={protocolOptions}
            searchable={false}
            renderSelected={(val) => ({ displayValue: protocolToString(val), key: val })}
            disposable
          />
        </SectionWithCard>
        <SectionWithCard title="Denied Users/Devices/Groups" id="denied-card">
          <MessageBox styleVariant={MessageBoxStyleVariant.OUTLINED}>
            <p>
              Specify one or more fields (Users or Groups) to define this rule. The rule
              will consider all inputs provided for matching conditions. Leave any fields
              blank if not needed.
            </p>
          </MessageBox>
          <LabeledCheckbox
            label="Deny all users"
            value={denyAllUsers}
            onChange={(val) => {
              if (val) {
                setAllowAllUsers(false);
              }
              setDenyAllUsers(val);
            }}
          />
          <FormDialogSelect
            label="Users"
            controller={{ control, name: 'denied_users' }}
            options={users}
            renderTagContent={renderUserTag}
            renderDialogListItem={renderUserListItem}
            identKey="id"
            searchKeys={['username', 'first_name', 'last_name']}
            disabled={denyAllUsers}
          />
          <FormDialogSelect
            label="Groups"
            controller={{ control, name: 'denied_groups' }}
            options={groups}
            renderTagContent={renderGroup}
            identKey="id"
            searchKeys={['name']}
            disabled={denyAllUsers || true}
          />
          <FormDialogSelect
            label="Network Devices"
            controller={{ control, name: 'denied_devices' }}
            options={devices}
            renderTagContent={renderNetworkDevice}
            identKey="id"
            searchKeys={['name']}
          />
        </SectionWithCard>
        <input type="submit" ref={submitRef} className="hidden" />
      </form>
    </PageContainer>
  );
};

const CardHeader = ({ title }: { title: string }) => {
  return (
    <div className="header">
      <h3>{title}</h3>
      <hr />
    </div>
  );
};

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

const renderAlias = (alias: Alias) => <p>{alias.name}</p>;

const renderGroup = (group: GroupInfo) => <p>{group.name}</p>;

const protocolToString = (value: AclProtocol): string => {
  switch (value) {
    case AclProtocol.TCP:
      return 'TCP';
    case AclProtocol.UDP:
      return 'UDP';
    case AclProtocol.ICMP:
      return 'ICMP';
  }
};

const protocolOptions: SelectOption<number>[] = [
  {
    key: AclProtocol.TCP,
    label: 'TCP',
    value: AclProtocol.TCP,
  },
  {
    key: AclProtocol.UDP,
    label: 'UDP',
    value: AclProtocol.UDP,
  },
  {
    key: AclProtocol.ICMP,
    label: 'ICMP',
    value: AclProtocol.ICMP,
  },
];
