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
    const res = [<Link to="/acl/aliases" key={0}>{`Aliases`}</Link>];
    if (isEdit) {
      res.push(
        <Link
          to="/acl/edit-alias"
          search={{
            alias: alias?.id as number,
          }}
          key={1}
        >{`Edit alias`}</Link>,
      );
    } else {
      res.push(<Link to="/acl/add-alias" key={1}>{`Add new alias`}</Link>);
    }
    return res;
  }, [alias?.id, isEdit]);

  return (
    <EditPage
      pageTitle="Aliases"
      links={breadcrumbs}
      headerProps={{
        icon: 'add-alias',
        title: isEdit ? `Edit alias` : `Add new alias`,
        subtitle: `ACL alias functionality allows administrators to create reusable elements which can then be used when defining a destination in multiple ACL rules. You must define at least one element in the alias settings.`,
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
      } else {
        await addAlias(toSend);
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
            {(field) => <field.FormInput required label="Alias name" />}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="location-tracking">
          <DescriptionBlock title="Addresses/Ranges">
            <p>{`Define the IP addresses or ranges that form the destination of this ACL rule.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="addresses">
            {(field) => (
              <field.FormInput
                notNull
                label={`IPv4/IPv6 CIDR ranges or addresses (or multiple values separated by commas)`}
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title="Ports">
            <p>{`You may specify the exact ports accessible to users in this location.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="ports">
            {(field) => (
              <field.FormInput
                notNull
                label={`Manually defined ports (or multiple values separated by commas)`}
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title="Protocols">
            <p>{`By default, all protocols are allowed for this location. You can change this configuration, but at least one protocol must be selected.`}</p>
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
                        text={isEdit ? 'Edit alias' : 'Add alias'}
                        loading={isSubmitting}
                        disabled={isEmpty}
                      />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>
                    <p>{`At least one component is required.`}</p>
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
