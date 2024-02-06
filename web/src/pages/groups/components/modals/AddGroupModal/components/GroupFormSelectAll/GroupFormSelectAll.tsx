import { useCallback } from 'react';
import { Control, useController, useWatch } from 'react-hook-form';

import { SelectRow } from '../../../../../../../shared/defguard-ui/components/Layout/SelectRow/SelectRow';
import { User } from '../../../../../../../shared/types';
import { ModifyGroupFormFields } from '../../AddGroupModal';

type Props = {
  // how many users are there
  control: Control<ModifyGroupFormFields>;
  users: User[];
};

export const GroupFormSelectAll = ({ users, control }: Props) => {
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
      <p>Select all users</p>
    </SelectRow>
  );
};
