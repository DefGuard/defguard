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
      <clipPath id="icon-packets-out_svg__a">
        <path d="M0 0h16v16H0z" className="icon-packets-out_svg__a" />
      </clipPath>
      <style>{'.icon-packets-out_svg__a{fill:#cb3f3f}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-packets-out_svg__a)',
      }}
      transform="translate(1.997 5)"
    >
      <rect
        width={2}
        height={2}
        className="icon-packets-out_svg__a"
        rx={1}
        transform="translate(9 2)"
      />
      <rect
        width={2}
        height={2}
        className="icon-packets-out_svg__a"
        rx={1}
        transform="translate(6 2)"
      />
      <g
        style={{
          fill: '#cb3f3f',
        }}
      >
        <path
          d="M4.234 4H1.766L3 1.944z"
          style={{
            stroke: 'none',
          }}
          transform="rotate(-90 3 3)"
        />
        <path
          d="M3 .944c.332 0 .663.161.857.485l1.234 2.057A1 1 0 0 1 4.234 5H1.766A1 1 0 0 1 .91 3.486l1.234-2.057A.99.99 0 0 1 3 .944"
          style={{
            stroke: 'none',
            fill: '#cb3f3f',
          }}
          transform="rotate(-90 3 3)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconPacketsOut;
