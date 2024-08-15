import { useCallback } from 'react';
import { Control, useController, useWatch } from 'react-hook-form';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { SelectRow } from '../../../../../../../shared/defguard-ui/components/Layout/SelectRow/SelectRow';
import { User } from '../../../../../../../shared/types';
import { ModifyGroupFormFields } from '../../AddGroupModal';

type Props = {
  // how many users are there
  control: Control<ModifyGroupFormFields>;
  users: User[];
};

export const GroupFormSelectAll = ({ users, control }: Props) => {
  const { LL } = useI18nContext();
  const {
    field: { value, onChange },
  } = useController({ control, name: 'members' });

  const membersValue = useWatch({ control, name: 'members' });

  const handleSelect = useCallback(() => {
    if (value.length !== users.length) {
      onChange(users.map((u) => u.username));
      return;
    }
    onChange([]);
  }, [onChange, users, value.length]);

  return (
    <SelectRow
      selected={(membersValue?.length ?? 0) === users.length}
      className="select-all"
      onClick={() => handleSelect()}
    >
      <p>{LL.modals.addGroup.selectAll()}</p>
    </SelectRow>
  );
};
