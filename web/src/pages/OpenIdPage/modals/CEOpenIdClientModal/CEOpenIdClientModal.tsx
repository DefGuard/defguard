import z from 'zod';
import {
  type AddOpenIdClient,
  type OpenIdClient,
  OpenIdClientScope,
  type OpenIdClientScopeValue,
} from '../../../../shared/api/types';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenCEOpenIdClientModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { DescriptionBlock } from '../../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';

const modalNameValue = ModalName.CEOpenIdClient;

type ModalData = OpenCEOpenIdClientModal;

export const CeOpenIdClientModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="ce-openid-client-modal"
      title={'Add OpenID application'}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const getScopeLabel = (value: OpenIdClientScopeValue): string => {
  switch (value) {
    case 'email':
      return m.cmp_openid_scope_email();
    case 'groups':
      return m.cmp_openid_scope_groups();
    case 'openid':
      return m.cmp_openid_scope_openid();
    case 'phone':
      return m.cmp_openid_scope_phone();
    case 'profile':
      return m.cmp_openid_scope_profile();
  }
};

const availableScopes = Object.values(OpenIdClientScope);

const ModalContent = ({ reservedNames, openIdClient }: ModalData) => {
  const [scopes, setScopes] = useState<Set<OpenIdClientScopeValue>>(
    new Set(openIdClient?.scope ?? []),
  );

  const isEdit = isPresent(openIdClient);

  const { mutateAsync: addClient } = useMutation({
    mutationFn: api.openIdClient.addOpenIdClient,
    meta: {
      invalidate: ['oauth'],
    },
  });

  const { mutateAsync: editClient } = useMutation({
    mutationFn: api.openIdClient.editOpenIdClient,
    meta: {
      invalidate: ['oauth'],
    },
  });

  const toggleScope = useCallback(
    (scope: OpenIdClientScopeValue) => {
      const isIn = scopes.has(scope);
      if (isIn) {
        const modified = new Set(scopes);
        modified.delete(scope);
        setScopes(modified);
      } else {
        const modified = new Set(scopes);
        modified.add(scope);
        setScopes(modified);
      }
    },
    [scopes],
  );

  const formSchema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .refine((value) => {
            if (value === openIdClient?.name) return true;
            return !reservedNames.includes(value.toLowerCase());
          }),
        redirect_uri: z.array(
          z.url(m.form_error_invalid()).min(1, m.form_error_required()),
        ),
      }),
    [openIdClient?.name, reservedNames],
  );
  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo((): FormFields => {
    if (openIdClient) {
      return {
        name: openIdClient.name,
        redirect_uri: openIdClient.redirect_uri,
      };
    }
    return {
      name: '',
      redirect_uri: [''],
    };
  }, [openIdClient]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      if (isEdit) {
        const data: OpenIdClient = {
          ...openIdClient,
          ...value,
          scope: Array.from(scopes),
        };
        await editClient(data);
      } else {
        const data: AddOpenIdClient = {
          ...value,
          enabled: true,
          scope: Array.from(scopes),
        };
        await addClient(data);
      }
      closeModal(modalNameValue);
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <form.AppField name="name">
          {(field) => (
            <field.FormInput label={m.modal_ce_openid_client_label_name()} required />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Lg} />
        <form.AppField name="redirect_uri" mode="array">
          {(field) => {
            return (
              <>
                <div className="fields">
                  {field.state.value.map((_, index) => (
                    <form.AppField key={index} name={`redirect_uri[${index}]`}>
                      {(subField) => (
                        <subField.FormInput
                          required
                          label={m.modal_ce_openid_client_label_redirect({
                            index: index + 1,
                          })}
                          onDismiss={
                            index !== 0
                              ? (e) => {
                                  e.preventDefault();
                                  e.stopPropagation();
                                  form.removeFieldValue('redirect_uri', index, {
                                    dontValidate: true,
                                    dontUpdateMeta: true,
                                    dontRunListeners: true,
                                  });
                                }
                              : undefined
                          }
                        />
                      )}
                    </form.AppField>
                  ))}
                </div>
                <SizedBox height={ThemeSpacing.Md} />
                <button
                  type="button"
                  className="add-url"
                  data-testid="add-url"
                  onClick={() => {
                    field.pushValue('', {
                      dontValidate: true,
                    });
                  }}
                >
                  <Icon icon="plus" />
                  <span>{m.modal_ce_openid_client_label_redirect_add()}</span>
                </button>
              </>
            );
          }}
        </form.AppField>
        <Divider spacing={ThemeSpacing.Xl} />
        <DescriptionBlock title={m.modal_ce_openid_client_label_scopes_title()}>
          <p>{m.test_placeholder_long()}</p>
        </DescriptionBlock>
        <SizedBox height={ThemeSpacing.Xl} />
        <div className="scopes">
          {availableScopes.map((scope) => (
            <Checkbox
              key={scope}
              active={scopes.has(scope)}
              text={getScopeLabel(scope)}
              testId={`field-scope-${getScopeLabel(scope).toLowerCase()}`}
              onClick={() => {
                toggleScope(scope);
              }}
            />
          ))}
        </div>
        <ModalControls
          cancelProps={{
            disabled: isSubmitting,
            text: m.controls_cancel(),
            onClick: () => {
              closeModal(modalNameValue);
            },
          }}
          submitProps={{
            text: isPresent(openIdClient)
              ? m.controls_save_changes()
              : m.controls_submit(),
            testId: 'save-settings',
            loading: isSubmitting,
            onClick: () => {
              form.handleSubmit();
            },
          }}
        />
      </form.AppForm>
    </form>
  );
};
