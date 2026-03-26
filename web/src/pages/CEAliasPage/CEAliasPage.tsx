import { useMutation } from '@tanstack/react-query';
import { Link, useRouter } from '@tanstack/react-router';
import { cloneDeep } from 'lodash-es';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type AclAlias,
  AclProtocol,
  AclProtocolName,
  type AclProtocolValue,
  type AddAclAliasRequest,
  aclProtocolValues,
} from '../../shared/api/types';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { aclDestinationValidator, aclPortsValidator } from '../../shared/validators';

const getProtocolLabel = (protocol: AclProtocolValue) => AclProtocolName[protocol];

interface Props {
  alias?: AclAlias;
}

export const CEAliasPage = ({ alias }: Props) => {
  const isEdit = useMemo(() => isPresent(alias), [alias]);

  const breadcrumbs = useMemo(() => {
    const res = [
      <Link to="/acl/aliases" key={0}>
        {m.cmp_nav_item_aliases()}
      </Link>,
    ];
    if (isEdit) {
      res.push(
        <Link
          to="/acl/edit-alias"
          search={{
            alias: alias?.id as number,
          }}
          key={1}
        >
          {m.acl_alias_form_title_edit()}
        </Link>,
      );
    } else {
      res.push(
        <Link to="/acl/add-alias" key={1}>
          {m.acl_alias_form_title_add()}
        </Link>,
      );
    }
    return res;
  }, [alias?.id, isEdit]);

  return (
    <EditPage
      pageTitle={m.cmp_nav_item_aliases()}
      links={breadcrumbs}
      headerProps={{
        icon: 'add-alias',
        title: isEdit ? m.acl_alias_form_title_edit() : m.acl_alias_form_title_add(),
        subtitle: m.acl_alias_form_subtitle(),
      }}
    >
      <FormContent alias={alias} />
    </EditPage>
  );
};

const formSchema = z.object({
  name: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  ports: aclPortsValidator,
  addresses: aclDestinationValidator,
  protocols: z.set(z.enum(AclProtocol)),
});

type FormFields = z.infer<typeof formSchema>;

const anyComponentDefined = (fields: FormFields): boolean => {
  return (
    fields.ports.trim().length > 0 ||
    fields.addresses.trim().length > 0 ||
    fields.protocols.size > 0
  );
};

const FormContent = ({ alias }: { alias?: AclAlias }) => {
  const isEdit = isPresent(alias);

  const defaultValues = useMemo((): FormFields => {
    if (isPresent(alias)) {
      return {
        name: alias.name,
        addresses: alias.addresses,
        ports: alias.ports,
        protocols: new Set(alias.protocols),
      };
    }
    return {
      name: '',
      addresses: '',
      ports: '',
      protocols: new Set(),
    };
  }, [alias]);

  const router = useRouter();

  const { mutateAsync: addAlias } = useMutation({
    mutationFn: api.acl.alias.addAlias,
    meta: {
      invalidate: ['acl', 'alias'],
    },
  });

  const { mutateAsync: editAlias } = useMutation({
    mutationFn: api.acl.alias.editAlias,
    meta: {
      invalidate: ['acl', 'alias'],
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const toSend: AddAclAliasRequest = {
        ...cloneDeep(value),
        protocols: Array.from(value.protocols),
      };

      if (isPresent(alias)) {
        await editAlias({ ...toSend, id: alias.id });
        Snackbar.default(m.acl_alias_updated_pending());
      } else {
        await addAlias(toSend);
        Snackbar.default(m.acl_alias_created());
      }

      router.history.back();
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <MarkedSection icon="settings">
          <form.AppField name="name">
            {(field) => <field.FormInput required label={m.acl_alias_col_name()} />}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="location-tracking">
          <DescriptionBlock title={m.acl_form_section_addresses_title()}>
            <p>{m.acl_form_section_addresses_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="addresses">
            {(field) => <field.FormInput notNull label={m.acl_form_addresses_label()} />}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title={m.acl_form_section_ports_title()}>
            <p>{m.acl_form_section_ports_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="ports">
            {(field) => <field.FormInput notNull label={m.acl_form_ports_label()} />}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title={m.acl_form_section_protocols_title()}>
            <p>{m.acl_form_section_protocols_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="protocols">
            {(field) => (
              <field.FormCheckboxGroup
                values={aclProtocolValues}
                getLabel={getProtocolLabel}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Lg} />
        </MarkedSection>
        <form.Subscribe
          selector={(s) => ({
            isDefault: s.isDefaultValue || s.isPristine,
            isSubmitting: s.isSubmitting,
            isEmpty: !anyComponentDefined(s.values),
          })}
        >
          {({ isSubmitting, isEmpty }) => (
            <Controls>
              <div className="right">
                <Button
                  variant="secondary"
                  text={m.controls_cancel()}
                  onClick={() => {
                    router.history.back();
                  }}
                />
                <TooltipProvider disabled={!isEmpty}>
                  <TooltipTrigger>
                    <div>
                      <Button
                        type="submit"
                        text={
                          isEdit ? m.controls_save_changes() : m.acl_alias_action_add()
                        }
                        loading={isSubmitting}
                        disabled={isEmpty}
                      />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{m.acl_alias_form_component_required()}</p>
                  </TooltipContent>
                </TooltipProvider>
              </div>
            </Controls>
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
