## 📖 Description

1. include a **summary of the changes and the related issue**, eg. _Closes #XYZ_
2. Do not make a PR if you can't check **all the boxes below**

### 🛠️ Dev Branch Merge Checklist:

#### Documentation ###

- [ ] If testing requires changes in the environment or deployment, please **update the documentation** (https://defguard.gitbook.io) first and **attach the link to the documentation** section in this pool request
- [ ] I have commented on my code, particularly in hard-to-understand areas

#### Testing ### 

- [ ] I have prepared end-to-end tests for all new functionalities
- [ ] I have performed end-to-end tests manually and they work
- [ ] New and existing unit tests pass locally with my changes

#### Deployment ###

- [ ] If deployment is affected I have made corresponding/required changes to [deployment](https://github.com/defguard/deployment) (Docker, Kubernetes, one-line install)

### 🏚️ Main Branch Merge Checklist:

#### Testing ### 

- [ ] I have merged my changes before to dev and the dev checklist is done
- [ ] I have tested all functionalities on the dev instance and they work

#### Documentation ###

- [ ] I have made corresponding changes to the **user & admin documentation** and added new features documentation with screenshots for users/admins
