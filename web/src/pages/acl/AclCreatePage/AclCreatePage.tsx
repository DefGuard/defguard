import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMemo, useState } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../i18n/i18n-react';
import { PageContainer } from '../../../shared/components/Layout/PageContainer/PageContainer';
import { SectionWithCard } from '../../../shared/components/Layout/SectionWithCard/SectionWithCard';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ActivityIcon } from '../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { LabeledCheckbox } from '../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { Network } from '../../../shared/types';
import { useAclLoadedContext } from '../acl-context';
import { FormDialogSelect } from './components/DialogSelect/FormDialogSelect';

export const AlcCreatePage = () => {
  const { LL } = useI18nContext();
  const localLL = LL.acl.createPage;
  const formErrors = LL.form.error;
  const { networks } = useAclLoadedContext();
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
        ports: z.string({
          required_error: formErrors.required(),
        }),
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
        <SectionWithCard title={localLL.sections.rule.title()}>
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
