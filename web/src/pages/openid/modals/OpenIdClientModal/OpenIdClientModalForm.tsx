import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useFieldArray, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ActionButton } from '../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { patternValidUrl } from '../../../../shared/patterns';
import { QueryKeys } from '../../../../shared/queries';
import { OpenIdClientModalFormScopes } from './components/OpenIdClientModalFormScopes';
import { OpenIdClientFormFields, OpenIdClientScope } from './types';

const defaultValuesEmptyForm: OpenIdClientFormFields = {
  name: '',
  redirect_uri: [{ url: '' }],
  scope: [],
};

export const OpenIdClientModalForm = () => {
  const { writeToClipboard } = useClipboard();
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
        scope: modalState.client.scope as OpenIdClientScope[],
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
          LL.openidOverview.modals.openidClientModal.form.messages.successAdd(),
        );
        setModalState({ visible: false });
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false });
        console.error(err);
      },
    },
  );
  const { mutate: editMutation, isLoading: editLoading } = useMutation(
    [MutationKeys.EDIT_OPENID_CLIENT],
    editOpenidClient,
    {
      onSuccess: () => {
        toaster.success(
          LL.openidOverview.modals.openidClientModal.form.messages.successModify(),
        );
        setModalState({ visible: false });
        queryClient.invalidateQueries([QueryKeys.FETCH_CLIENTS]);
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        setModalState({ visible: false });
        console.error(err);
      },
    },
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
                LL.openidOverview.modals.openidClientModal.form.error.urlRequired(),
              )
              .test('valid-url', (val, ctx) => {
                if (val.includes('localhost')) return true;
                if (!patternValidUrl.test(val)) {
                  return ctx.createError({
                    message:
                      LL.openidOverview.modals.openidClientModal.form.error.validUrl(),
                  });
                }
                return true;
              }),
          })
          .required(),
      ),
      scope: yup.array(yup.string()),
    })
    .required();

  const { handleSubmit, control } = useForm<OpenIdClientFormFields>({
    defaultValues: defaultFormValues,
    mode: 'all',
    resolver: yupResolver(schema),
  });

  const { fields, remove, append } = useFieldArray({
    control,
    name: 'redirect_uri',
  });

  const onValidSubmit: SubmitHandler<OpenIdClientFormFields> = (values) => {
    if (modalState.viewMode) return;
    if (values.scope.length === 0) {
      toaster.error(
        LL.openidOverview.modals.openidClientModal.form.error.scopeValidation(),
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
    [modalState.viewMode],
  );

  return (
    <form onSubmit={handleSubmit(onValidSubmit)} data-testid="openid-client-form">
      <FormInput
        controller={{ control, name: 'name' }}
        label={LL.openidOverview.modals.openidClientModal.form.fields.name.label()}
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
            label={LL.openidOverview.modals.openidClientModal.form.fields.redirectUri.label(
              { count: index + 1 },
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
            size={ButtonSize.LARGE}
            text={LL.openidOverview.modals.openidClientModal.form.controls.addUrl()}
            onClick={() => append({ url: '' })}
          />
        )}
      </div>
      <h3>{LL.openidOverview.modals.openidClientModal.scopes()}</h3>
      <OpenIdClientModalFormScopes control={control} disabled={modalState.viewMode} />
      {modalState.viewMode && !isUndefined(modalState.client) && (
        <div className="client-info">
          <ExpandableCard
            disableExpand={false}
            title={LL.openidOverview.modals.openidClientModal.clientId()}
            actions={[
              <ActionButton
                data-testid="copy-client-id"
                key={1}
                variant={ActionButtonVariant.COPY}
                onClick={() => {
                  if (modalState.client) {
                    writeToClipboard(
                      modalState.client.client_id,
                      LL.openidOverview.modals.openidClientModal.messages.clientIdCopy(),
                    );
                  }
                }}
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
                disabled={isUndefined(modalState.client)}
                onClick={() => {
                  if (modalState.client) {
                    writeToClipboard(
                      modalState.client.client_secret,
                      LL.openidOverview.modals.openidClientModal.messages.clientSecretCopy(),
                    );
                  }
                }}
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
          size={ButtonSize.LARGE}
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
            size={ButtonSize.LARGE}
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
