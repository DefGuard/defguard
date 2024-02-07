import type { SVGProps } from 'react';
const SvgIconListOrderUp = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-list-order-up_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#899ca8',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-list-order-up_svg__c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-list-order-up_svg__a)',
      }}
      transform="rotate(-90 11 11)"
    >
      <g transform="rotate(90 4 12)">
        <rect
          width={14}
          height={2}
          className="icon-list-order-up_svg__c"
          rx={1}
          transform="translate(0 8)"
        />
        <rect
          width={10}
          height={2}
          className="icon-list-order-up_svg__c"
          rx={1}
          transform="translate(0 4)"
        />
        <rect width={10} height={2} className="icon-list-order-up_svg__c" rx={1} />
      </g>
      <g transform="translate(6)">
        <rect
          width={8}
          height={2}
          className="icon-list-order-up_svg__c"
          rx={1}
          transform="translate(0 2)"
        />
        <g
          style={{
            fill: '#899ca8',
          }}
        >
          <path
            d="M4.234 4H1.766L3 1.944z"
            style={{
              stroke: 'none',
            }}
            transform="rotate(90 5.5 5.5)"
          />
          <path
            d="M3 .944c.332 0 .663.161.857.485l1.234 2.057A1 1 0 0 1 4.234 5H1.766A1 1 0 0 1 .91 3.486l1.234-2.057A.99.99 0 0 1 3 .944"
            style={{
              stroke: 'none',
              fill: '#899ca8',
            }}
            transform="rotate(90 5.5 5.5)"
          />
        </g>
      </g>
    </g>
  </svg>
);
export default SvgIconListOrderUp;
