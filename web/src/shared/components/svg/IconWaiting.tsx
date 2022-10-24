import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconWaiting = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-waiting_svg__a">
        <path className="icon-waiting_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-waiting_svg__a,.icon-waiting_svg__c{fill:#899ca8}.icon-waiting_svg__a{opacity:0}.icon-waiting_svg__b{clip-path:url(#icon-waiting_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-waiting_svg__b">
      <path
        className="icon-waiting_svg__c"
        d="m14.207 12.59-2.51-1.882V6.874a.7.7 0 0 0-1.394 0v4.183a.7.7 0 0 0 .279.558l2.789 2.091a.7.7 0 0 0 .837-1.115Z"
      />
      <path
        className="icon-waiting_svg__c"
        d="M11 2a9 9 0 1 0 9 9 9.01 9.01 0 0 0-9-9Zm0 16.606A7.606 7.606 0 1 1 18.606 11 7.615 7.615 0 0 1 11 18.606Z"
      />
    </g>
  </svg>
);

export default SvgIconWaiting;
