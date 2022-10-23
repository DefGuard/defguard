import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconUserAddNew = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-user-add-new_svg__a">
        <path className="icon-user-add-new_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-user-add-new_svg__a,.icon-user-add-new_svg__c{fill:#fff}.icon-user-add-new_svg__a{opacity:0}.icon-user-add-new_svg__b{clip-path:url(#icon-user-add-new_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-user-add-new_svg__b">
      <path
        className="icon-user-add-new_svg__c"
        d="M2.933 21a.781.781 0 0 1-.781-.781 7.118 7.118 0 0 1 7.11-7.11h1.172a7.1 7.1 0 0 1 2.341.395.781.781 0 0 1-.514 1.475 5.537 5.537 0 0 0-1.827-.308H9.262a5.553 5.553 0 0 0-5.547 5.547.781.781 0 0 1-.782.782Zm12.11-14.726a5.274 5.274 0 1 0-5.274 5.274 5.28 5.28 0 0 0 5.274-5.274Zm-1.563 0a3.711 3.711 0 1 1-3.711-3.711 3.715 3.715 0 0 1 3.712 3.711Zm5.586 10.04h-2.343v-2.345a.781.781 0 0 0-1.563 0v2.344h-2.343a.781.781 0 1 0 0 1.563h2.344v2.344a.781.781 0 0 0 1.563 0v-2.344h2.344a.781.781 0 0 0 0-1.563Z"
      />
    </g>
  </svg>
);

export default SvgIconUserAddNew;
