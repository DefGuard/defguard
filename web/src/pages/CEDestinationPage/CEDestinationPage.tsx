import { useMutation } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type AclDestination,
  AclProtocol,
  AclProtocolName,
  type AclProtocolValue,
  aclProtocolValues,
} from '../../shared/api/types';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { InfoBanner } from '../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { aclDestinationValidator, aclPortsValidator } from '../../shared/validators';

type Props = {
  destination?: AclDestination;
};

const formSchema = z
  .object({
    name: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
    addresses: aclDestinationValidator,
    ports: aclPortsValidator,
    protocols: z.set(z.enum(AclProtocol)),
    any_address: z.boolean(),
    any_port: z.boolean(),
    any_protocol: z.boolean(),
  })
  .superRefine((values, ctx) => {
    if (!values.any_address && values.addresses.trim().length === 0) {
      ctx.addIssue({
        code: 'custom',
        continue: true,
        path: ['addresses'],
        message: m.form_error_required(),
      });
    }
    if (!values.any_port && values.ports.trim().length === 0) {
      ctx.addIssue({
        code: 'custom',
        continue: true,
        path: ['ports'],
        message: m.form_error_required(),
      });
    }
    if (!values.any_protocol && values.protocols.size === 0) {
      ctx.addIssue({
        code: 'custom',
        continue: false,
        path: ['protocols'],
        message: m.form_error_required(),
      });
    }
  });

type FormFields = z.infer<typeof formSchema>;

const getProtocolName = (value: AclProtocolValue): string => AclProtocolName[value];

export const CEDestinationPage = ({ destination }: Props) => {
  const router = useRouter();
  const isEdit = isPresent(destination);

  const { mutateAsync: addDestination } = useMutation({
    mutationFn: api.acl.destination.addDestination,
    onSuccess: () => {
      Snackbar.success('Destination added');
    },
    onError: (e) => {
      Snackbar.error('Error occurred');
      console.error(e);
    },
    meta: {
      invalidate: ['acl', 'destination'],
    },
  });

  const { mutateAsync: editDestination } = useMutation({
    mutationFn: api.acl.destination.editDestination,
    onSuccess: () => {
      Snackbar.success('Destination modified');
    },
    onError: (e) => {
      Snackbar.error('Error occurred');
      console.error(e);
    },
    meta: {
      invalidate: ['acl', 'destination'],
    },
  });

  const defaultValues = useMemo((): FormFields => {
    if (isPresent(destination)) {
      return {
        name: destination.name,
        any_address: true,
        any_port: true,
        any_protocol: true,
        addresses: destination.addresses,
        ports: destination.ports,
        protocols: new Set(destination.protocols),
      };
    }

    return {
      name: '',
      ports: '',
      any_address: true,
      any_port: true,
      any_protocol: true,
      addresses: '',
      protocols: new Set(),
    };
  }, [destination]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const toSend = { ...value, protocols: Array.from(value.protocols) };
      try {
        if (isPresent(destination)) {
          await editDestination({
            ...toSend,
            id: destination.id,
          });
        } else {
          await addDestination(toSend);
        }
        router.history.back();
      } catch (e) {
        console.error(e);
      }
    },
  });

  return (
    <EditPage
      pageTitle="Destinations"
      headerProps={{
        title: isEdit ? 'Edit destination' : 'Add destination',
        icon: 'add-location',
        subtitle: `ACL alias functionality allows administrators to create reusable elements which can then be used when defining a destination in multiple ACL rules. You must define at least one element in the alias settings.`,
      }}
    >
      {(destination?.rules?.length ?? 0) > 0 && (
        <>
          <InfoBanner
            variant="warning"
            icon="info-outlined"
            text={`This destination is linked to one or more active ACL rules. Any changes you make here after the destination is deployed will also affect the rules that depend on it.`}
          />
          <SizedBox height={ThemeSpacing.Xl2} />
        </>
      )}
      <form.AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            e.stopPropagation();
            form.handleSubmit();
          }}
        >
          <MarkedSection icon="settings">
            <form.AppField name="name">
              {(field) => <field.FormInput required label="Destination name" />}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="location-tracking">
            <DescriptionBlock title="Addresses/Ranges">
              <p>{`Define the IP addresses or ranges that form the destination of this ACL rule.`}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField name="any_address">
              {(field) => <field.FormToggle label="All IP addresses" />}
            </form.AppField>
            <form.Subscribe selector={(s) => !s.values.any_address}>
              {(open) => (
                <Fold open={open}>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <form.AppField name="addresses">
                    {(field) => (
                      <field.FormTextarea
                        required
                        placeholder="ex. 192.168.12.1, 192.23.56.12, 198.156.23.12"
                        label="IPv4/IPv6 CIDR ranges or addresses (or multiple values separated by commas)"
                      />
                    )}
                  </form.AppField>
                </Fold>
              )}
            </form.Subscribe>
            <Divider spacing={ThemeSpacing.Xl} />
            <DescriptionBlock title="Ports">
              <p>{`You may specify the exact ports accessible to users in this location.`}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField name="any_port">
              {(field) => <field.FormToggle label="All ports" />}
            </form.AppField>
            <form.Subscribe selector={(s) => !s.values.any_port}>
              {(open) => (
                <Fold open={open}>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <form.AppField name="ports">
                    {(field) => (
                      <field.FormInput
                        required
                        label="Manually defined ports (or multiple values separated by commas)"
                      />
                    )}
                  </form.AppField>
                </Fold>
              )}
            </form.Subscribe>
            <Divider spacing={ThemeSpacing.Xl} />
            <DescriptionBlock title="Protocols">
              <p>{`By default, all protocols are allowed for this location. You can change this configuration, but at least one protocol must remain selected.`}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField name="any_protocol">
              {(field) => <field.FormToggle label="All protocols" />}
            </form.AppField>
            <form.Subscribe selector={(s) => !s.values.any_protocol}>
              {(open) => (
                <Fold open={open}>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <form.AppField name="protocols">
                    {(field) => (
                      <field.FormCheckboxGroup
                        values={aclProtocolValues}
                        getLabel={getProtocolName}
                      />
                    )}
                  </form.AppField>
                </Fold>
              )}
            </form.Subscribe>
          </MarkedSection>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Controls>
            <div className="right">
              <Button
                text={m.controls_cancel()}
                variant="secondary"
                onClick={() => {
                  router.history.back();
                }}
              />
              <Button
                variant="primary"
                text={isEdit ? 'Save changes' : 'Add destination'}
                type="submit"
              />
            </div>
          </Controls>
        </form>
      </form.AppForm>
    </EditPage>
  );
};
