import type { SVGProps } from 'react';
const SvgIconPacketsOut = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path d="M0 0h16v16H0z" className="a" />
      </clipPath>
      <style>{'.a{fill:#cb3f3f}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="translate(1.997 5)"
    >
      <rect width={2} height={2} className="a" rx={1} transform="translate(9 2)" />
      <rect width={2} height={2} className="a" rx={1} transform="translate(6 2)" />
      <g
        style={{
          fill: '#cb3f3f',
        }}
      >
        <path
          d="M4.234 4H1.766L3 1.944 4.234 4Z"
          style={{
            stroke: 'none',
          }}
          transform="rotate(-90 3 3)"
        />
        <path
          d="M3 .944c.332 0 .663.161.857.485l1.234 2.057A1 1 0 0 1 4.234 5H1.766A1 1 0 0 1 .91 3.486l1.234-2.057A.991.991 0 0 1 3 .944Z"
          style={{
            fill: '#cb3f3f',
            stroke: 'none',
          }}
          transform="rotate(-90 3 3)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconPacketsOut;
