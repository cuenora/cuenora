//! Read-only recurrence projection; never invokes generation or mutates coverage.
use crate::{
    account_creation::today_at,
    account_reports::{begin, money, unavailable},
    api_error::ApiError,
    auth_identity::AuthenticatedIdentity,
    profile_zone::parse_zone,
};
use chrono::{DateTime, Datelike, Days, NaiveDate, SecondsFormat, Utc};
use cuenora_domain::{BusinessDate, RecurrenceCadence, next_occurrence};
use serde::Serialize;
use sqlx::PgPool;

const MAX_RESOURCES: usize = 1000;
const MAX_SLOTS: usize = 10000;
type RuleRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
);
type ProjectionRow = (Option<String>, String, String, bool);
type PlanningRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    bool,
);
#[derive(Serialize)]
pub(crate) struct Planning {
    as_of: String,
    through: String,
    complete: bool,
    accounts: Vec<AccountPlan>,
    unassigned: Vec<Unassigned>,
}
#[derive(Serialize)]
struct AccountPlan {
    account_id: String,
    name: String,
    account_type: String,
    status: String,
    currency: String,
    current_balance: String,
    pending_total: String,
    projected_total: String,
    planning_difference: Option<String>,
    complete: bool,
}
#[derive(Serialize)]
struct Unassigned {
    currency: String,
    pending_total: String,
    projected_total: String,
    complete: bool,
}
fn date(value: &str) -> Result<BusinessDate, ApiError> {
    BusinessDate::parse(value).map_err(|_| ApiError::AccountReportUnavailable)
}
pub(crate) async fn read(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    requested: Option<String>,
) -> Result<Planning, ApiError> {
    let (mut tx, zone) = begin(pool, identity).await?;
    // Trusted instant captured once after establishing the read snapshot. All rule
    // local days and the optional profile-zone default derive from this instant.
    let now: DateTime<Utc> = std::time::SystemTime::now().into();
    let through = match requested {
        Some(value) => date(&value)?,
        None => {
            let zone = parse_zone(zone.as_deref().ok_or(ApiError::ProfileZoneUnconfirmed)?)?;
            let today = today_at(now, zone)
                .map_err(|_| ApiError::DateOutOfRange)?
                .to_string();
            let day = NaiveDate::parse_from_str(&today, "%Y-%m-%d")
                .map_err(|_| ApiError::DateOutOfRange)?;
            let end = (1..=31)
                .rev()
                .find_map(|d| NaiveDate::from_ymd_opt(day.year(), day.month(), d))
                .ok_or(ApiError::DateOutOfRange)?;
            date(&end.to_string())?
        }
    };
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id::text FROM accounts WHERE user_id=$1::uuid LIMIT 1001")
            .bind(identity.user_id())
            .fetch_all(&mut *tx)
            .await
            .map_err(unavailable)?;
    if ids.len() > MAX_RESOURCES {
        return Err(ApiError::PlanningTooComplex);
    }
    let rules:Vec<RuleRow>=sqlx::query_as("SELECT id::text,cadence,to_char(starts_on,'YYYY-MM-DD'),to_char(ends_on,'YYYY-MM-DD'),to_char(generated_through,'YYYY-MM-DD'),time_zone,state,to_char(paused_on,'YYYY-MM-DD') FROM recurring_rules WHERE user_id=$1::uuid ORDER BY id LIMIT 1001")
        .bind(identity.user_id()).fetch_all(&mut *tx).await.map_err(unavailable)?;
    if rules.len() > MAX_RESOURCES {
        return Err(ApiError::PlanningTooComplex);
    }
    let mut rule_ids = Vec::new();
    let mut slots = Vec::new();
    let mut future = Vec::new();
    for (id, cadence, anchor, end, covered, zone, state, paused_on) in rules {
        let today = today_at(now, parse_zone(&zone)?).map_err(|_| ApiError::DateOutOfRange)?;
        let mut end = end
            .as_deref()
            .map(date)
            .transpose()?
            .map_or(through, |e| e.min(through));
        if state == "paused" {
            // Pausing stops projections, not detection of eligible missing slots
            // before the pause. Those slots may be backfilled after a resume.
            let pause = paused_on
                .as_deref()
                .ok_or(ApiError::AccountReportUnavailable)?;
            let pause = NaiveDate::parse_from_str(pause, "%Y-%m-%d")
                .map_err(|_| ApiError::AccountReportUnavailable)?;
            let Some(previous) = pause.checked_sub_days(Days::new(1)) else {
                continue;
            };
            // A pause on the first supported date has no earlier eligible slot.
            if previous.year() < 1 {
                continue;
            }
            end = end.min(date(&previous.to_string())?).min(today);
        }
        let mut after = covered.as_deref().map(date).transpose()?;
        let anchor = date(&anchor)?;
        let cadence = match cadence.as_str() {
            "weekly" => RecurrenceCadence::Weekly,
            "monthly" => RecurrenceCadence::Monthly,
            "quarterly" => RecurrenceCadence::Quarterly,
            "annual" => RecurrenceCadence::Annual,
            _ => return Err(ApiError::AccountReportUnavailable),
        };
        while let Some(slot) = next_occurrence(anchor, cadence, after) {
            if slot > end {
                break;
            }
            if slots.len() == MAX_SLOTS {
                return Err(ApiError::PlanningTooComplex);
            }
            rule_ids.push(id.clone());
            slots.push(slot.to_string());
            future.push(slot > today);
            after = Some(slot);
        }
    }
    // Native multirange exclusions preserve calendar-edit/pause coverage. Any
    // persisted state wins its scheduled slot, including postponed/skipped items.
    let projections:Vec<ProjectionRow>=sqlx::query_as(
        "SELECT r.account_id::text,r.currency,
            coalesce(sum(r.amount_minor::numeric) FILTER (WHERE s.future),0)::text,bool_or(NOT s.future)
        FROM unnest($2::text[],$3::text[],$4::bool[]) s(rule_id,day,future)
        JOIN recurring_rules r ON r.id=s.rule_id::uuid AND r.user_id=$1::uuid
        WHERE NOT (r.excluded_dates @> s.day::date)
            AND NOT EXISTS(SELECT 1 FROM recurring_occurrences o WHERE o.user_id=$1::uuid
                AND o.rule_id=r.id AND o.scheduled_on=s.day::date)
        GROUP BY r.account_id,r.currency",
    ).bind(identity.user_id()).bind(rule_ids).bind(slots).bind(future).fetch_all(&mut *tx).await.map_err(unavailable)?;
    let mut accounts = Vec::new();
    let mut currencies = Vec::new();
    let mut amounts = Vec::new();
    let mut incomplete = Vec::new();
    for (account, currency, amount, missing) in projections {
        accounts.push(account);
        currencies.push(currency);
        amounts.push(amount);
        incomplete.push(missing);
    }
    // Aggregate pending independently of rule state/current terms. Union unassigned
    // currency keys explicitly; no account gets its obligations implicitly assigned.
    let rows:Vec<PlanningRow>=sqlx::query_as(
        "WITH projected AS (
            SELECT p.account::uuid AS account_id,p.currency,p.amount::numeric AS amount,p.incomplete
            FROM unnest($2::text[],$3::text[],$4::text[],$5::bool[]) p(account,currency,amount,incomplete)
        ), pending AS (
            SELECT account_id,currency,sum(amount_minor) AS amount FROM recurring_occurrences
            WHERE user_id=$1::uuid AND state='planned' AND due_on<=$6::date GROUP BY account_id,currency
        ), keys AS (
            SELECT id AS account_id,name,account_type,CASE WHEN archived_at IS NULL THEN 'active' ELSE 'archived' END AS status,currency
            FROM accounts WHERE user_id=$1::uuid
            UNION ALL SELECT NULL::uuid,NULL::text,NULL::text,NULL::text,c FROM (VALUES ('SEK'),('EUR'),('USD')) u(c)
        ), balances AS (
            SELECT account_id,currency,sum(signed_amount_minor) AS amount FROM account_movements
            WHERE user_id=$1::uuid GROUP BY account_id,currency
        )
        SELECT k.account_id::text,k.name,k.account_type,k.status,k.currency,
            CASE WHEN k.account_id IS NOT NULL THEN coalesce(b.amount,0)::text END,
            coalesce(n.amount,0)::text,coalesce(p.amount,0)::text,
            CASE WHEN k.account_id IS NOT NULL AND NOT coalesce(p.incomplete,false)
                THEN (coalesce(b.amount,0)-coalesce(n.amount,0)-coalesce(p.amount,0))::text END,
            NOT coalesce(p.incomplete,false)
        FROM keys k LEFT JOIN pending n ON n.account_id IS NOT DISTINCT FROM k.account_id AND n.currency=k.currency
        LEFT JOIN projected p ON p.account_id IS NOT DISTINCT FROM k.account_id AND p.currency=k.currency
        LEFT JOIN balances b ON b.account_id=k.account_id AND b.currency=k.currency
        ORDER BY k.account_id NULLS LAST,CASE k.currency WHEN 'SEK' THEN 1 WHEN 'EUR' THEN 2 ELSE 3 END",
    ).bind(identity.user_id()).bind(accounts).bind(currencies).bind(amounts).bind(incomplete).bind(through.to_string())
        .fetch_all(&mut *tx).await.map_err(unavailable)?;
    let mut accounts = Vec::new();
    let mut unassigned = Vec::new();
    let mut complete = true;
    for (id, name, kind, status, currency, balance, pending, projected, difference, covered) in rows
    {
        complete &= covered;
        if let Some(account_id) = id {
            accounts.push(AccountPlan {
                account_id,
                name: name.ok_or(ApiError::AccountReportUnavailable)?,
                account_type: kind.ok_or(ApiError::AccountReportUnavailable)?,
                status: status.ok_or(ApiError::AccountReportUnavailable)?,
                currency,
                current_balance: money(
                    balance
                        .as_deref()
                        .ok_or(ApiError::AccountReportUnavailable)?,
                )?,
                pending_total: money(&pending)?,
                projected_total: money(&projected)?,
                planning_difference: difference.as_deref().map(money).transpose()?,
                complete: covered,
            });
        } else {
            unassigned.push(Unassigned {
                currency,
                pending_total: money(&pending)?,
                projected_total: money(&projected)?,
                complete: covered,
            });
        }
    }
    tx.commit().await.map_err(unavailable)?;
    Ok(Planning {
        as_of: now.to_rfc3339_opts(SecondsFormat::Micros, true),
        through: through.to_string(),
        complete,
        accounts,
        unassigned,
    })
}
