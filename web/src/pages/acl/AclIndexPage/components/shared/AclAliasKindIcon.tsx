import { AclAliasKind } from '../../../types';

type Props = {
  kind: AclAliasKind;
};
export const AclAliasKindIcon = ({ kind }: Props) => {
  switch (kind) {
    case AclAliasKind.COMPONENT:
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
        >
          <circle cx="8" cy="5" r="1" fill="#0C8CE0" />
          <circle cx="8" cy="11" r="1" fill="#0C8CE0" />
          <circle cx="5" cy="9" r="1" fill="#0C8CE0" />
          <circle cx="11" cy="9" r="1" fill="#0C8CE0" />
        </svg>
      );
    case AclAliasKind.DESTINATION:
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
        >
          <circle cx="5" cy="8" r="1" transform="rotate(-90 5 8)" fill="#0C8CE0" />
          <circle cx="9" cy="11" r="1" transform="rotate(-90 9 11)" fill="#0C8CE0" />
          <circle cx="9" cy="5" r="1" transform="rotate(-90 9 5)" fill="#0C8CE0" />
          <path
            d="M9 11L11 8L9 5"
            stroke="#0C8CE0"
            strokeWidth="0.5"
            strokeLinecap="round"
          />
          <path d="M11 8L5 8" stroke="#0C8CE0" strokeWidth="0.5" strokeLinecap="round" />
        </svg>
      );
  }
};
