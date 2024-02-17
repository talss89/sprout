import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

import tailwind from "@astrojs/tailwind";

// https://astro.build/config
export default defineConfig({
	site: 'https://talss89.github.io',
	base: '/sprout',
	integrations: [starlight({
		title: 'Sprout',
		customCss: [
			'./src/styles/tailwind.css'
		],
		social: {
			github: 'https://github.com/talss89/sprout'
		},
		sidebar: [
			{
				label: 'Installation',
				link: '/install'
			},
			{
				label: 'Using Sprout',
				link: '/using-sprout'
			},
			{
				label: 'Guides',
				items: [
					// Each item here is one entry in the navigation menu.
					{
						label: 'Creating a repo on S3',
						link: '/guides/creating-a-repo-on-s3/'
					}]
			}, {
				label: 'Reference',
				autogenerate: {
					directory: 'reference'
				}
			}]
	}), tailwind({ applyBaseStyles: false })]
});