# GPO Insight

<img src="gpo-insight.png" alt="GPO Insight" width="200"/>

GPO Insight accepts GPO Exports created using the Get-GPOReport CMDLET in Active Directory.  
The following is an example of the command used to create the GPO Export.  
  
```powershell
Get-GPOReport -All -Domain "domain.com" -Server "ACME-DC1" -ReportType HTML -Path "C:\GPOReport.html"
```
## Usage
Execute parsing and analysis functionality on the file `GPOReport.html`.
```
gpo-insight -i GPOReport.html
```  
Specify GPO Exports using the `-i` (input) flag. No default value exists for this flag.  
Once the GPO Export is imported to GPO-Insight, the GPOs will be broken down into individual files that reflect each individual GPO.  
Outputs from GPO-Insight will be generated in the directory specified by the `-o` (output) flag, or will default to the Present Working Directory.  
GPOs are broken down into both `HTML` and `TXT` files.  
The `TXT` outputs are cleaned up `HTML2TXT` outputs of the `HTML` files.  

Only the `TXT` outputs are used to analyze GPOs.  
GPO Insight uses it's own "GPO Query Syntax" to specify desirable, undesirable, and warning search criteria.  
A directory named `queries` in the same directory as the GPO Insight EXE or the current working directory needs to exist to perform analysis. All of the individual files in the `queries` directory needs to follow GPO Query Syntax.
Completed analysis can be found in the output `analysis.txt` created in the directory specified by the `-o` flag.
    
## GPO Query Syntax
GPO Insight uses "GPO Query Syntax" to search GPOs to find GPOs that match the given criteria.
To create a gpo query, the following syntax elements may be used.
Be sure to follow the syntax closely, as anything that does not follow the syntax will be assumed by GPO-Insight to be a comment.
  
`Name::Value` where `Value` is the name of the GPO.  
  
`Details::Owner` where `Owner` is the value of the GPO's owner.  
  
`Links::Location` where `Location` is the Organizational Unit of the link.  
  
`Filtering::Value` where `Value` is the Security Filtering member.  
  
`Delegation::Name::Permissions::Inherited` where:  
- `Name` is the user or group associated with the delegation.  
- `Permissions` is the permission of the associated user or group (i.e. Read, Edit, Delete, Modify).  
- `Inherited` is if the Delegation's permission is inherited.  
  
`Policy::Value::Setting` where:  
- `Value` is the policy being set.  
- `Setting` is the configured setting for the policy.  
  
## Syntax Modifiers, Specifics, & Examples
  
Syntax modifiers give the base GPO Query Syntax more flexibility in searching GPOs.  
Syntax modifiers should always be applied to the beginning of the option.
  
### Name
#### Modifiers
  
No modifiers can be applied.
  
#### Examples
Match Example
```
Name::Test GPO
```  
  
### Details
#### Modifiers
  
The **Owner** value for the Details query syntax can apply the following modifiers.  
  
| Modifier | Description |
| --- | --- |
| `>` | "Ends With"  |
| `<` | "Starts With" |
| `!` | "Is Not" |
  
#### Examples
Match Example
```
Details::LABS\\Domain Admins
```
Ends With Example
```
Details::>Domain Admins
```
Starts With Example
```
Details::<LABS
```  
Is Not Example  
```
Details::!>Domain Admins
```
  
### Links
#### Modifiers
  
The **Location** value for the Links query syntax can apply the following modifiers.  
  
| Modifier | Description |
| --- | --- |
| `>` | "Ends With"  |
| `<` | "Starts With" |
  
#### Examples
Match Example
```
Links::Domain Controllers
```  
Ends With Example
```
Links::>Users
```
Starts With Example
```
Links::<Domain
```
  
### Filtering
#### Modifiers
  
No modifiers can be applied.
  
#### Examples
Match Example
```
Filtering::NT AUTHORITY\\Authenticated Users
```  
  
### Delegation
#### Modifiers
  
The **Name** and **Permissions** values for the Delegation query syntax can apply the following modifiers.
  
| Modifier | Description |
| --- | --- |
| `>` | "Ends With"  |
| `<` | "Starts With" |  
  
#### Notes
    
Additionally, the **Inherited** value can be left blank if it is unimportant to the condition.  
However, if the **Inherited** value is left blank, you cannot ommit the trailing `::` idenfitiers.
  
#### Examples
Match Example
```
Delegation::NT AUTHORITY\\Authenticated Users::Read (from Security Filtering)::
```  
Ends With Example(s)
```
Delegation::>Authenticated Users::>(from Security Filtering)::
```
Starts With Example(s)
```
Delegation::<NT AUTHORITY::<Read::
```
  
### Policy
#### Modifiers
  
The **Value** and **Setting** values for the Policy query syntax can apply the following modifiers.  
  
| Modifier | Description |
| --- | --- |
| `>` | "Ends With"  |
| `<` | "Starts With" |

The **Setting** values for the Policy query syntax can apply the following additional modifiers.  
  
| Modifier | Description |
| --- | --- |
| `#>=` | "Numerical Greater-Than-or-Equal-To" |
| `#>` | "Numerical Greater-Than" |
| `#<=` | "Numerical Less-Than-or-Equal-To" |
| `#<` | "Numerical Less-Than" |
| `!` | "Is Not" |
  
Numerical modifiers should only contain numbers and no spaces, or the modifer will break the condition.  
  
The "Is Not" modifier for policy settings should only be used for registry values that can be a single value -- **Lists will result in unintended behaviors.** For example, if the intention of a query is to ensure that ONLY the Domain Admins group could debug a program, the following query would fail the purpose: `Policy::Debug program::!>Domain Admins`. This is because if anyone is added to the permission that is not Domain Admin, but the Domain Admins remains in the permission, the query will fail to trigger because Domain Admins IS in the setting's list. Future versions will aim to improve and add additional "is not" functionality.

#### Notes
  
Additionally, the **Setting** value can be left blank if it is unimportant to the condition.  
However, if the **Setting** value is left blank, you cannot ommit the trailing `::` idenfitiers.
  
#### Examples
Match Example
```
Policy::Debug programs::NT AUTHORITY\\Authenticated Users
```  
Ends With Example
```
Policy::Debug programs::>Authenticated Users
```  
Starts With Example
```
Policy::<Debug::<NT AUTHORITY
```
Numerical Greater-Than-Or-Equal-To Example
```
Policy::Minimum password length::#>=14
```
Numerical Less-Than Example
```
Policy::Minimum password length::#<14
```
Is Not Example
```
Policy::Debug Programs::!>Domain Admins
```

## Using GPO Query Syntax in queries files
`Queries` files should prepend their GPO Query Syntax with one of the following:
| Flag | Description |
| `D -- ` | The Query identifies a desirable GPO value. |
| `U -- ` | The Query identifies an undesirable GPO value. |
| `W -- ` | The Query identifies a GPO value that may or may not be undesirable. |
| `M -- ` | The Query identifies a missing GPO value that should exist. |

## Combining Queries
Queries using the GPO Query Syntax can be combined into compound conditions using the "|" (pipe) symbol.
  
#### Examples
Compound query using a Link condition and a Policy condition
```
Links::Domain Controllers | Policy::Domain controller: LDAP server signing requirements::None
```

## Comments
Any line in the configuration files that does not match the GPO Query Syntax will be considered a comment.  
Additionally, all valid lines of GPO Query Syntax can use "//" to end the line with a comment.
#### Examples
```
// This is a comment
This is a comment, as well.
Links::Domain Controllers | Policy::Domain controller: LDAP server signing requirements::None // This is a comment after a valid query
```  
## Missing Features
Missing features that are intended to be added in future versions.  
- [ ] Policy Parsing for Registries  

## Acknowledgements
Thanks to the team at Black Hills Information Security (BHIS) that have contributed to this tool by sharing their knowledge and insights of Active Directory.  

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.  

## License
[GPLv3](https://www.gnu.org/licenses/)  
