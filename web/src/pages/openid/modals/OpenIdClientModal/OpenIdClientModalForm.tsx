import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import clipboard from 'clipboardy';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useFieldArray, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../shared/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import {
  ActionButton,
  ActionButtonVariant,
} from '../../../../shared/components/layout/ActionButton/ActionButton';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { ExpandableCard } from '../../../../shared/components/layout/ExpandableCard/ExpandableCard';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { patternValidUrl } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';

const defaultValuesEmptyForm: FormInputs = {
  name: '',
  redirect_uri: [{ url: '' }],
  scope: [],
};

export const OpenIdClientModalForm = () => {
  const { LL } = useI18nContext();
  const {
    openid: { addOpenidClient, editOpenidClient },
  } = useApi();
  const toaster = useToaster();
  const modalState = useModalStore((state) => state.openIdClientModal);
  const setModalState = useModalStore((state) => state.setOpenIdClientModal);
  const queryClient = useQueryClient();
  const defaultFormValues = useMemo(() => {
    if (modalState.client) {
      const urls = modalState.client.redirect_uri.map((u) => ({ url: u }));
      return {
        name: modalState.client.name,
        redirect_uri: urls,
        scope: modalState.client.scope,
      };
    }
    return defaultValuesEmptyForm;
  }, [modalState.client]);

  const { mutate: addMutation, isLoading: addLoading } = useMutation(
    [MutationKeys.ADD_OPENID_CLIENT],
    addOpenidClient,
    {
      onSuccess: () => {
        toaster.success(
          LL.openidOverview.modals.openidClientModal.form.messages.successAdd()
        );
        setModalState({ visible: false });
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false });
        console.error(err);
      },
    }
  );
  const { mutate: editMutation, isLoading: editLoading } = useMutation(
    [MutationKeys.EDIT_OPENID_CLIENT],
    editOpenidClient,
    {
      onSuccess: () => {
        toaster.success(
          LL.openidOverview.modals.openidClientModal.form.messages.successModify()
        );
        setModalState({ visible: false });
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false });
        console.error(err);
      },
    }
  );
  const schema = yup
    .object()
    .shape({
      name: yup
        .string()
        .required()
        .min(4, LL.form.error.minimumLength())
        .max(16, LL.form.error.maximumLength()),
      redirect_uri: yup.array().of(
        yup
          .object()
          .shape({
            url: yup
              .string()
              .required(
                LL.openidOverview.modals.openidClientModal.form.error.urlRequired()
              )
              .matches(
                patternValidUrl,
                LL.openidOverview.modals.openidClientModal.form.error.validUrl()
              ),
          })
          .required()
      ),
      scope: yup.array(yup.string()),
    })
    .required();

  const { handleSubmit, control } = useForm<FormInputs>({
    defaultValues: defaultFormValues,
    mode: 'all',
    resolver: yupResolver(schema),
  });

  const { fields, remove, append } = useFieldArray({
    control,
    name: 'redirect_uri',
  });

  const onValidSubmit: SubmitHandler<FormInputs> = (values) => {
    if (modalState.viewMode) return;
    if (values.scope.length === 0) {
      toaster.error(
        LL.openidOverview.modals.openidClientModal.form.error.scopeValidation()
      );
      return;
    }
    const urls = values.redirect_uri.map((u) => u.url);
    if (modalState.client) {
      editMutation({
        ...modalState.client,
        ...values,
        redirect_uri: urls,
      });
    } else {
      addMutation({
        name: values.name,
        scope: values.scope,
        redirect_uri: urls,
        enabled: true,
      });
    }
  };

  const getControlsClass = useMemo(
    () =>
      classNames('controls', {
        'view-mode': modalState.viewMode,
      }),
    [modalState.viewMode]
  );

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        outerLabel={LL.openidOverview.modals.openidClientModal.form.fields.name.label()}
        placeholder={LL.openidOverview.modals.openidClientModal.form.fields.name.label()}
        disabled={modalState.viewMode}
        required
      />
      <div className="urls">
        {fields.map((field, index) => (
          <FormInput
            key={field.id}
            controller={{ control, name: `redirect_uri.${index}.url` }}
            placeholder={LL.openidOverview.modals.openidClientModal.form.fields.redirectUri.placeholder()}
            outerLabel={LL.openidOverview.modals.openidClientModal.form.fields.redirectUri.label(
              { count: index + 1 }
            )}
            disposable
            disposeHandler={() => remove(index)}
            disabled={modalState.viewMode}
            required
          />
        ))}
        {!modalState.viewMode && (
          <Button
            styleVariant={ButtonStyleVariant.PRIMARY}
            size={ButtonSize.BIG}
            text={LL.openidOverview.modals.openidClientModal.form.controls.addUrl()}
            onClick={() => append({ url: '' })}
          />
        )}
      </div>
      <h3>{LL.openidOverview.modals.openidClientModal.scopes()}</h3>
      <div className="scopes">
        <FormCheckBox
          label={LL.openidOverview.modals.openidClientModal.form.fields.openid.label()}
          disabled={modalState.viewMode}
          labelPosition="right"
          controller={{ control, name: 'scope' }}
          customValue={(context: OpenIdScope[]) =>
            !isUndefined(context.find((scope) => scope === OpenIdScope.OPENID))
          }
          customOnChange={(context: OpenIdScope[]) => {
            const exist = !isUndefined(
              context.find((scope) => scope === OpenIdScope.OPENID)
            );
            if (exist) {
              return context.filter((s) => s !== OpenIdScope.OPENID);
            }
            return [...context, OpenIdScope.OPENID];
          }}
        />
        <FormCheckBox
          disabled={modalState.viewMode}
          label={LL.openidOverview.modals.openidClientModal.form.fields.profile.label()}
          labelPosition="right"
          controller={{ control, name: 'scope' }}
          customValue={(context: OpenIdScope[]) =>
            !isUndefined(context.find((scope) => scope === OpenIdScope.PROFILE))
          }
          customOnChange={(context: OpenIdScope[]) => {
            const exist = !isUndefined(
              context.find((scope) => scope === OpenIdScope.PROFILE)
            );
            if (exist) {
              return context.filter((s) => s !== OpenIdScope.PROFILE);
            }
            return [...context, OpenIdScope.PROFILE];
          }}
        />
        <FormCheckBox
          disabled={modalState.viewMode}
          label={LL.openidOverview.modals.openidClientModal.form.fields.email.label()}
          labelPosition="right"
          controller={{ control, name: 'scope' }}
          customValue={(context: OpenIdScope[]) =>
            !isUndefined(context.find((scope) => scope === OpenIdScope.EMAIL))
          }
          customOnChange={(context: OpenIdScope[]) => {
            const exist = !isUndefined(
              context.find((scope) => scope === OpenIdScope.EMAIL)
            );
            if (exist) {
              return context.filter((s) => s !== OpenIdScope.EMAIL);
            }
            return [...context, OpenIdScope.EMAIL];
          }}
        />
        <FormCheckBox
          disabled={modalState.viewMode}
          label={LL.openidOverview.modals.openidClientModal.form.fields.phone.label()}
          labelPosition="right"
          controller={{ control, name: 'scope' }}
          customValue={(context: OpenIdScope[]) =>
            !isUndefined(context.find((scope) => scope === OpenIdScope.PHONE))
          }
          customOnChange={(context: OpenIdScope[]) => {
            const exist = !isUndefined(
              context.find((scope) => scope === OpenIdScope.PHONE)
            );
            if (exist) {
              return context.filter((s) => s !== OpenIdScope.PHONE);
            }
            return [...context, OpenIdScope.PHONE];
          }}
        />
      </div>
      {modalState.viewMode && !isUndefined(modalState.client) && (
        <div className="client-info">
          <ExpandableCard
            disableExpand={false}
            title={LL.openidOverview.modals.openidClientModal.clientId()}
            actions={[
              <ActionButton
                key={1}
                variant={ActionButtonVariant.COPY}
                onClick={() =>
                  clipboard
                    .write(modalState.client ? modalState.client.client_id : '')
                    .then(() => {
                      toaster.success(
                        LL.openidOverview.modals.openidClientModal.messages.clientIdCopy()
                      );
                    })
                    .catch((err) => {
                      toaster.error(LL.messages.clipboardError());
                      console.error(err);
                    })
                }
              />,
            ]}
          >
            <p>{modalState.client.client_id}</p>
          </ExpandableCard>
          <ExpandableCard
            disableExpand={false}
            title={LL.openidOverview.modals.openidClientModal.clientSecret()}
            actions={[
              <ActionButton
                key={1}
                variant={ActionButtonVariant.COPY}
                onClick={() =>
                  clipboard
                    .write(modalState.client ? modalState.client.client_secret : '')
                    .then(() => {
                      toaster.success(
                        LL.openidOverview.modals.openidClientModal.messages.clientSecretCopy()
                      );
                    })
                    .catch((err) => {
                      toaster.error(LL.messages.clipboardError());
                      console.error(err);
                    })
                }
              />,
            ]}
          >
            <p>{modalState.client.client_secret}</p>
          </ExpandableCard>
        </div>
      )}
      <div className={getControlsClass}>
        <Button
          text={modalState.viewMode ? LL.form.close() : LL.form.cancel()}
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.BIG}
          onClick={() =>
            setModalState({
              visible: false,
            })
          }
          type="button"
          className="cancel"
        />
        {!modalState.viewMode && (
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={LL.form.submit()}
            type="submit"
            className="submit"
            disabled={modalState.viewMode}
            loading={addLoading || editLoading}
          />
        )}
      </div>
    </form>
  );
};

enum OpenIdScope {
  OPENID = 'openid',
  PROFILE = 'profile',
  EMAIL = 'email',
  PHONE = 'phone',
}

type FormInputs = {
  name: string;
  redirect_uri: {
    url: string;
  }[];
  scope: string[];
};
