pub static WELCOME_MESSAGE: &str = "Dear {{ first_name }} {{ last_name }},

By completing the enrollment process, you now have access to all company systems.

Your login to all systems is: {{ username }}

## Company systems

Here are the most important company systems:

- Defguard: {{ defguard_url }} - where you can change your password and manage your VPN devices
- our chat system: https://chat.example.com - join our default room #TownHall
- knowledge base: https://example.com ...
- our JIRA: https://example.atlassian.net...

## Governance

To kickoff your onboarding, please get familiar with:

- our employee handbook: https://knowledgebase.example.com/Welcome
- security policy: https://knowledgebase.example.com/security

If you have any questions contact our HR:
John Hary - mobile +48 123 123 123

The person that enrolled you is:
{{ admin_first_name }} {{ admin_last_name }},
email: {{ admin_email }}
mobile: {{ admin_phone }}

--
Sent by Defguard {{ defguard_version }}
Star us on GitHub! https://github.com/defguard/defguard\
";

pub static WELCOME_EMAIL_SUBJECT: &str = "Defguard: Welcome message after enrollment";
