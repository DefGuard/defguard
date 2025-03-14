import { shallow } from 'zustand/shallow';

import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { AlcAliasCEModal } from './modals/AlcAliasCEModal/AlcAliasCEModal';
import { useAclAliasCEModal } from './modals/AlcAliasCEModal/store';

export const AclIndexAliases = () => {
  const openCEModal = useAclAliasCEModal((s) => s.open, shallow);
  return (
    <div id="acl-aliases">
      <header>
        <h2>Aliases</h2>
        <div className="controls">
          <Button
            className="filter"
            text="Filters"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.LINK}
            icon={
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="18"
                height="18"
                viewBox="0 0 18 18"
                fill="none"
              >
                <path
                  d="M15.5455 3.27026C15.5455 3.07996 15.4699 2.89745 15.3353 2.76288C15.2007 2.62832 15.0182 2.55272 14.8279 2.55272H3.17211C3.04054 2.55262 2.91148 2.58869 2.79903 2.65699C2.68658 2.7253 2.59507 2.8232 2.53452 2.94001C2.47396 3.05681 2.44668 3.18802 2.45567 3.31928C2.46466 3.45054 2.50956 3.5768 2.58547 3.68426L6.81138 9.69299L6.82365 14.0645C6.825 14.3153 6.89413 14.5611 7.02372 14.7758C7.15331 14.9905 7.33854 15.1662 7.5598 15.2842C7.78107 15.4023 8.03014 15.4583 8.28065 15.4464C8.53115 15.4345 8.77378 15.3551 8.98284 15.2165L10.4924 14.2102C10.6889 14.0783 10.8497 13.8998 10.9605 13.6907C11.0713 13.4815 11.1286 13.2482 11.1273 13.0115L11.1117 9.72163L15.4129 3.68426C15.4989 3.56329 15.5452 3.41865 15.5455 3.27026ZM9.67911 9.26181L9.69629 13.0115L8.25793 13.9729L8.24484 9.23563L4.55402 3.98699H13.437L9.67911 9.26181Z"
                  fill="#485964"
                />
              </svg>
            }
          />
          <Button
            text="Add new"
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            onClick={() => {
              openCEModal();
            }}
            icon={
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="18"
                height="18"
                viewBox="0 0 18 18"
                fill="none"
              >
                <path
                  d="M4.5 9H13.5"
                  stroke="white"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
                <path
                  d="M9 4.5L9 13.5"
                  stroke="white"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </svg>
            }
          />
        </div>
      </header>
      <AliasesList />
      <AlcAliasCEModal />
    </div>
  );
};

export const AliasesList = () => {
  return (
    <div id="aliases-list">
      <div className="headers"></div>
      <div className="list">
        <ul>
          <li className="cell"></li>
        </ul>
      </div>
    </div>
  );
};
