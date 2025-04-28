import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { omit } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { SelectOption } from '../../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../../shared/queries';
import { AclAliasKind } from '../../../../../types';
import { protocolOptions, protocolToString } from '../../../../../utils';
import { aclDestinationValidator, aclPortsValidator } from '../../../../../validators';
import { useAclAliasCEModal } from './store';

export const AlcAliasCEModal = () => {
  const isOpen = useAclAliasCEModal((s) => s.visible);
  const alias = useAclAliasCEModal((s) => s.alias);
  const isEdit = isPresent(alias);

  const [close, reset] = useAclAliasCEModal((s) => [s.close, s.reset], shallow);

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      id="acl-alias-ce-modal"
      title={isEdit ? 'Edit Alias' : 'Create Alias'}
      isOpen={isOpen}
      onClose={() => {
        close();
      }}
      afterClose={() => {
        reset();
      }}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const queryClient = useQueryClient();
  const closeModal = useAclAliasCEModal((s) => s.close, shallow);
  const initialAlias = useAclAliasCEModal((s) => s.alias);
  const isEditMode = isPresent(initialAlias);
  const toaster = useToaster();

  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.aliases.modals.create;
  const {
    acl: {
      aliases: { createAlias, editAlias },
    },
  } = useApi();

  const schema = useMemo(
    () =>
      z.object({
        name: z.string(),
        kind: z.string(),
        ports: aclPortsValidator(LL),
        destination: aclDestinationValidator(LL),
        protocols: z.number().array(),
      }),
    [LL],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo((): FormFields => {
    let defaultValues: FormFields;
    if (isPresent(initialAlias)) {
      defaultValues = omit(initialAlias, ['id']);
    } else {
      defaultValues = {
        destination: '',
        name: '',
        kind: '',
        ports: '',
        protocols: [],
      };
    }
    return defaultValues;
  }, [initialAlias]);

  const {
    handleSubmit,
    control,
    formState: { isSubmitting },
  } = useForm<FormFields>({
    mode: 'all',
    resolver: zodResolver(schema),
    defaultValues,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    try {
      if (isEditMode) {
        await editAlias({
          ...values,
          id: initialAlias.id,
        });
        toaster.success(localLL.messages.modified());
      } else {
        await createAlias(values);
        toaster.success(localLL.messages.created());
      }
      await queryClient.invalidateQueries({
        predicate: (query) => query.queryKey.includes(QueryKeys.FETCH_ACL_ALIASES),
      });
      closeModal();
    } catch (e) {
      toaster.error(LL.messages.error());
      console.error(e);
    }
    closeModal();
  };

  const aliasKindOptions = useMemo(
    (): SelectOption<string>[] => [
      {
        key: AclAliasKind.DESTINATION,
        value: AclAliasKind.DESTINATION,
        label: localLL.kindOptions.destination(),
      },
      {
        key: AclAliasKind.COMPONENT,
        value: AclAliasKind.COMPONENT,
        label: localLL.kindOptions.component(),
      },
    ],
    [localLL.kindOptions],
  );

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput controller={{ control, name: 'name' }} label={localLL.labels.name()} />
      <FormSelect
        controller={{ control, name: 'kind' }}
        label={localLL.labels.kind()}
        options={aliasKindOptions}
        searchable={false}
      />
      <div className="header">
        <h2>Destination</h2>
      </div>
      <FormInput
        controller={{ control, name: 'destination' }}
        label={localLL.labels.ip()}
        placeholder={localLL.placeholders.ip()}
      />
      <FormInput controller={{ control, name: 'ports' }} label={localLL.labels.ports()}
        placeholder={localLL.placeholders.ports()}
      />
      <FormSelect
        controller={{ control, name: 'protocols' }}
        label={localLL.labels.protocols()}
        placeholder={localLL.placeholders.protocols()}
        options={protocolOptions}
        searchable={false}
        renderSelected={(val) => ({ displayValue: protocolToString(val), key: val })}
        disposable
      />
      <div className="controls">
        <Button
          className="cancel"
          text={localLL.controls.cancel()}
          onClick={() => {
            closeModal();
          }}
          size={ButtonSize.LARGE}
          disabled={isSubmitting}
        />
        <Button
          className="submit"
          text={isEditMode ? localLL.controls.edit() : localLL.controls.create()}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          type="submit"
          loading={isSubmitting}
        />
      </div>
    </form>
  );
};
