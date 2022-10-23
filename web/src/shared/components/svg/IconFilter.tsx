import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconFilter = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-filter_svg__a">
        <path
          className="icon-filter_svg__a"
          transform="rotate(90 -303 906)"
          d="M0 0h22v22H0z"
        />
      </clipPath>
      <style>
        {
          '\n      .icon-filter_svg__a,.icon-filter_svg__c{fill:#899ca8}.icon-filter_svg__a{opacity:0}.icon-filter_svg__b{clip-path:url(#icon-filter_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-filter_svg__b">
      <path
        className="icon-filter_svg__c"
        d="M19 3.997a.877.877 0 0 0-.877-.877H3.877a.877.877 0 0 0-.717 1.383l5.165 7.344.015 5.343a1.7 1.7 0 0 0 2.639 1.408l1.845-1.23a1.753 1.753 0 0 0 .776-1.465l-.019-4.021 5.257-7.379A.877.877 0 0 0 19 3.997Zm-7.17 7.323.021 4.583-1.758 1.175-.016-5.79-4.511-6.415h10.857Z"
      />
    </g>
  </svg>
);

export default SvgIconFilter;
