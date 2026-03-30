import { useMutation } from '@tanstack/react-query';
import { useNavigate, useRouter } from '@tanstack/react-router';
import { omit } from 'radashi';
import { useCallback, useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import type { AclListTabValue } from '../../shared/aclTabs';
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
  tab?: AclListTabValue;
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

export const CEDestinationPage = ({ destination, tab }: Props) => {
  const router = useRouter();
  const navigate = useNavigate();
  const isEdit = isPresent(destination);
  const returnToDestinations = useCallback(() => {
    if (tab === undefined) {
      router.history.back();
      return;
    }

    navigate({
      to: '/acl/destinations',
      search: {
        tab,
      },
    });
  }, [navigate, router, tab]);

  const { mutateAsync: addDestination } = useMutation({
    mutationFn: api.acl.destination.addDestination,
    onError: () => {
      Snackbar.error(m.acl_destination_save_failed());
    },
    meta: {
      invalidate: ['acl', 'destination'],
    },
  });

  const { mutateAsync: editDestination } = useMutation({
    mutationFn: api.acl.destination.editDestination,
    onError: () => {
      Snackbar.error(m.acl_destination_save_failed());
    },
    meta: {
      invalidate: ['acl', 'destination'],
    },
  });

  const defaultValues = useMemo((): FormFields => {
    if (isPresent(destination)) {
      return {
        ...omit(destination, ['id', 'state', 'rules']),
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
          Snackbar.default(m.acl_destination_updated_pending());
        } else {
          await addDestination(toSend);
          Snackbar.default(m.acl_destination_created());
        }

        returnToDestinations();
      } catch {
        return;
      }
    },
  });

  return (
    <EditPage
      pageTitle={m.cmp_nav_item_destinations()}
      headerProps={{
        title: isEdit
          ? m.acl_destination_form_title_edit()
          : m.controls_add_destination(),
        icon: 'add-location',
        subtitle: m.acl_destination_form_subtitle(),
      }}
    >
      {(destination?.rules?.length ?? 0) > 0 && (
        <>
          <InfoBanner
            variant="warning"
            icon="info-outlined"
            text={m.acl_destination_active_rules_warning()}
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
              {(field) => (
                <field.FormInput
                  required
                  label={m.acl_destination_col_name()}
                  helper={m.acl_helper_destination_name()}
                />
              )}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="location-tracking">
            <DescriptionBlock title={m.acl_form_section_addresses_title()}>
              <p>{m.acl_form_section_addresses_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField name="any_address">
              {(field) => <field.FormToggle label={m.acl_destination_any_address()} />}
            </form.AppField>
            <form.Subscribe selector={(s) => !s.values.any_address}>
              {(open) => (
                <Fold open={open}>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <form.AppField name="addresses">
                    {(field) => (
                      <field.FormTextarea
                        required
                        placeholder={m.acl_form_addresses_placeholder()}
                        label={m.acl_form_addresses_label()}
                        helper={m.acl_helper_addresses()}
                      />
                    )}
                  </form.AppField>
                </Fold>
              )}
            </form.Subscribe>
            <Divider spacing={ThemeSpacing.Xl} />
            <DescriptionBlock title={m.acl_form_section_ports_title()}>
              <p>{m.acl_form_section_ports_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField name="any_port">
              {(field) => <field.FormToggle label={m.acl_destination_any_port()} />}
            </form.AppField>
            <form.Subscribe selector={(s) => !s.values.any_port}>
              {(open) => (
                <Fold open={open}>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <form.AppField name="ports">
                    {(field) => (
                      <field.FormInput
                        required
                        label={m.acl_form_ports_label()}
                        helper={m.acl_helper_ports()}
                      />
                    )}
                  </form.AppField>
                </Fold>
              )}
            </form.Subscribe>
            <Divider spacing={ThemeSpacing.Xl} />
            <DescriptionBlock title={m.acl_form_section_protocols_title()}>
              <p>{m.acl_form_section_protocols_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <form.AppField
              name="any_protocol"
              listeners={{
                onChange: ({ value, fieldApi }) => {
                  if (value) {
                    fieldApi.form.setFieldValue('protocols', new Set());
                  }
                },
              }}
            >
              {(field) => <field.FormToggle label={m.acl_destination_any_protocol()} />}
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
                  returnToDestinations();
                }}
              />
              <Button
                variant="primary"
                text={isEdit ? m.controls_save_changes() : m.controls_add_destination()}
                type="submit"
              />
            </div>
          </Controls>
        </form>
      </form.AppForm>
    </EditPage>
  );
};
