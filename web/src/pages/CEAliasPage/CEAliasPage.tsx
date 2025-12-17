import { useMutation } from '@tanstack/react-query';
import { Link, useRouter } from '@tanstack/react-router';
import { cloneDeep } from 'lodash-es';
import { useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type AclAlias,
  AclAliasKind,
  AclProtocol,
  AclProtocolName,
  type AddAclAliasRequest,
} from '../../shared/api/types';
import { CheckboxGroup } from '../../shared/components/CheckboxGroup/CheckboxGroup';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../shared/defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { aclDestinationValidator, aclPortsValidator } from '../../shared/validators';

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
  kind: z.enum(AclAliasKind),
  ports: aclPortsValidator,
  destination: aclDestinationValidator,
  protocols: z.set(z.enum(AclProtocol)),
});

type FormFields = z.infer<typeof formSchema>;

const FormContent = ({ alias }: { alias?: AclAlias }) => {
  const [allDestinations, setAllDestinations] = useState<boolean>(
    isPresent(alias) ? alias.destination.length === 0 : true,
  );
  const [allPorts, setAllPorts] = useState<boolean>(
    isPresent(alias) ? alias.ports.length === 0 : true,
  );
  const [allProtocols, setAllProtocols] = useState<boolean>(
    isPresent(alias) ? alias.protocols.length === 0 : true,
  );
  const defaultValues = useMemo((): FormFields => {
    if (isPresent(alias)) {
      return {
        name: alias.name,
        destination: alias.destination,
        kind: alias.kind,
        ports: alias.ports,
        protocols: new Set(alias.protocols),
      };
    }
    return {
      name: '',
      destination: '',
      kind: AclAliasKind.Component,
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
      if (allDestinations) {
        toSend.destination = '';
      }
      if (allProtocols) {
        toSend.protocols = [];
      }
      if (allPorts) {
        toSend.ports = '';
      }
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
          <Toggle
            active={allDestinations}
            label={`All IP addresses`}
            onClick={() => {
              setAllDestinations((s) => !s);
            }}
          />
          <Fold open={!allDestinations}>
            <SizedBox height={ThemeSpacing.Xl} />
            <form.AppField name="destination">
              {(field) => (
                <field.FormInput
                  notNull
                  label={`IPv4/IPv6 CIDR ranges or addresses (or multiple values separated by commas)`}
                />
              )}
            </form.AppField>
          </Fold>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title="Ports">
            <p>{`You may specify the exact ports accessible to users in this location.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <Toggle
            active={allPorts}
            label={`All ports`}
            onClick={() => {
              setAllPorts((s) => !s);
            }}
          />
          <Fold open={!allPorts}>
            <SizedBox height={ThemeSpacing.Xl} />
            <form.AppField name="ports">
              {(field) => (
                <field.FormInput
                  notNull
                  label={`Manually defined ports (or multiple values separated by commas)`}
                />
              )}
            </form.AppField>
          </Fold>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title="Protocols">
            <p>{`By default, all protocols are allowed for this location. You can change this configuration, but at least one protocol must be selected.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <Toggle
            active={allProtocols}
            label={`All protocols`}
            onClick={() => {
              setAllProtocols((s) => !s);
            }}
          />
          <Fold open={!allProtocols}>
            <SizedBox height={ThemeSpacing.Xl} />
            <CheckboxGroup>
              <form.AppField name="protocols">
                {(field) => (
                  <field.FormCheckbox
                    value={AclProtocol.UDP}
                    text={AclProtocolName[AclProtocol.UDP]}
                  />
                )}
              </form.AppField>
              <form.AppField name="protocols">
                {(field) => (
                  <field.FormCheckbox
                    value={AclProtocol.TCP}
                    text={AclProtocolName[AclProtocol.TCP]}
                  />
                )}
              </form.AppField>
              <form.AppField name="protocols">
                {(field) => (
                  <field.FormCheckbox
                    value={AclProtocol.ICMP}
                    text={AclProtocolName[AclProtocol.ICMP]}
                  />
                )}
              </form.AppField>
            </CheckboxGroup>
          </Fold>
        </MarkedSection>
        <form.Subscribe
          selector={(s) => ({
            isDefault: s.isDefaultValue || s.isPristine,
            isSubmitting: s.isSubmitting,
          })}
        >
          {({ isSubmitting }) => (
            <Controls>
              <div className="right">
                <Button
                  variant="secondary"
                  text={m.controls_cancel()}
                  onClick={() => {
                    router.history.back();
                  }}
                />
                <Button type="submit" text={'Add alias'} loading={isSubmitting} />
              </div>
            </Controls>
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
