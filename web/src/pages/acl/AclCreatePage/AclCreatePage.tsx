import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMemo, useState } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../i18n/i18n-react';
import { DateInput } from '../../../shared/components/Layout/DateInput/DateInput';
import { PageContainer } from '../../../shared/components/Layout/PageContainer/PageContainer';
import { SectionWithCard } from '../../../shared/components/Layout/SectionWithCard/SectionWithCard';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ActivityIcon } from '../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { LabeledCheckbox } from '../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { GroupInfo, Network, StandaloneDevice, User } from '../../../shared/types';
import { useAclLoadedContext } from '../acl-context';
import { FormDialogSelect } from './components/DialogSelect/FormDialogSelect';

type Alias = {
  id: number;
  name: string;
};

const mockedAliases: Alias[] = [];

export const AlcCreatePage = () => {
  const { LL } = useI18nContext();
  const localLL = LL.acl.createPage;
  const formErrors = LL.form.error;
  const { networks, devices, groups, users } = useAclLoadedContext();
  const [neverExpires, setNeverExpires] = useState(true);

  const schema = useMemo(
    () =>
      z.object({
        name: z
          .string({
            required_error: formErrors.required(),
          })
          .min(1, formErrors.required()),
        networks: z.number().array(),
        expires: z.string(),
        allow_all_users: z.boolean(),
        deny_all_users: z.boolean(),
        allowed_users: z.number().array(),
        denied_users: z.number().array(),
        allowed_groups: z.number().array(),
        denied_groups: z.number().array(),
        allowed_devices: z.number().array(),
        denied_devices: z.number().array(),
        aliases: z.number().array(),
        ports: z
          .string({
            required_error: formErrors.required(),
          })
          .min(1, formErrors.required())
          .regex(/^(?:\d+(?:-\d+)*)(?:[ ,]+\d+(?:-\d+)*)*$/, formErrors.invalid())
          .refine((value) => {
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
          .refine((value) => {
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
          }),
        protocols: z.string(),
      }),
    [formErrors],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      name: '',
      networks: [],
      aliases: [],
      ports: '',
      expires: '',
      protocols: '',
      allow_all_users: false,
      allowed_devices: [],
      allowed_groups: [],
      allowed_users: [],
      denied_devices: [],
      denied_groups: [],
      denied_users: [],
      deny_all_users: false,
    }),
    [],
  );

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    console.table(values);
  };

  const handleInvalidSubmit: SubmitErrorHandler<FormFields> = (errors) => {
    console.table(errors);
  };

  return (
    <PageContainer id="acl-create-page">
      <div className="header">
        <h1>{LL.acl.createPage.title()}</h1>
        <div className="controls"></div>
      </div>
      <form
        id="acl-sections"
        onSubmit={handleSubmit(handleValidSubmit, handleInvalidSubmit)}
      >
        <SectionWithCard title={localLL.sections.rule.title()} id="rule-card">
          <FormInput controller={{ control, name: 'name' }} label="Rule Name" />
          <FormDialogSelect
            controller={{ control, name: 'networks' }}
            options={networks}
            renderTagContent={renderNetworkSelectTag}
            identKey="id"
            label="Locations"
          />
          <CardHeader title="Expiration Date" />
          <LabeledCheckbox
            label="Never Expire"
            value={neverExpires}
            onChange={() => setNeverExpires((s) => !s)}
          />
          <DateInput />
        </SectionWithCard>
        <SectionWithCard title="Allowed Users/Devices/Groups" id="allow-card">
          <LabeledCheckbox label="All Active Users" value={false} onChange={() => {}} />
          <FormDialogSelect
            label="Users"
            controller={{ control, name: 'allowed_users' }}
            options={users}
            renderTagContent={renderUserTag}
            identKey="id"
          />
          <FormDialogSelect
            label="Groups"
            controller={{ control, name: 'allowed_groups' }}
            options={groups}
            renderTagContent={renderGroup}
            identKey="name"
          />
          <FormDialogSelect
            label="Network Devices"
            controller={{ control, name: 'allowed_devices' }}
            options={devices}
            renderTagContent={renderNetworkDevice}
            identKey="id"
          />
        </SectionWithCard>
        <SectionWithCard title="Destination" id="destination-card">
          <FormDialogSelect
            controller={{ control, name: 'aliases' }}
            options={mockedAliases}
            label="Aliases"
            identKey="id"
            renderTagContent={renderAlias}
          />
          <CardHeader title="Manual Input" />
          <FormInput controller={{ control, name: 'ports' }} label="Ports" />
          <FormInput controller={{ control, name: 'protocols' }} label="Protocols" />
        </SectionWithCard>
        <SectionWithCard title="Denied Users/Devices/Groups" id="denied-card">
          <LabeledCheckbox value={false} onChange={() => {}} label="All Active Users" />
          <FormDialogSelect
            label="Users"
            controller={{ control, name: 'denied_users' }}
            options={users}
            renderTagContent={renderUserTag}
            identKey="id"
          />
          <FormDialogSelect
            label="Groups"
            controller={{ control, name: 'denied_groups' }}
            options={groups}
            renderTagContent={renderGroup}
            identKey="name"
          />
          <FormDialogSelect
            label="Network Devices"
            controller={{ control, name: 'denied_devices' }}
            options={devices}
            renderTagContent={renderNetworkDevice}
            identKey="id"
          />
        </SectionWithCard>
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
    <ActivityIcon status={ActivityIconVariant.ERROR_FILLED} />
  </>
);

const renderUserTag = (user: User) => <p>{user.username}</p>;

const renderNetworkDevice = (device: StandaloneDevice) => <p>{device.name}</p>;

const renderAlias = (alias: Alias) => <p>{alias.name}</p>;

const renderGroup = (group: GroupInfo) => <p>{group.name}</p>;
