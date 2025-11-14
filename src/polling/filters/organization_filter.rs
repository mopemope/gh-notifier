use crate::polling::utils::extract_org_name;
use crate::{Config, Notification};

/// Filters notifications based on organization inclusion/exclusion rules
pub fn filter_by_organization(notification: &Notification, config: &Config) -> bool {
    let org_name = extract_org_name(&notification.repository.full_name);

    if !config
        .notification_filters()
        .include_organizations
        .is_empty()
        && !config
            .notification_filters()
            .include_organizations
            .contains(&org_name)
    {
        return false;
    }

    if config
        .notification_filters()
        .exclude_organizations
        .contains(&org_name)
    {
        return false;
    }

    true
}
