#pragma once

// WS-15.1.4: subscription widgets moved verbatim from bridge.hpp (move-only split).

#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QString>
#include <QStringList>
#include <QTableWidget>
#include <QWidget>

#include <map>
#include <utility>
#include <vector>

class HydrusMainWindow;
enum class UiAction : std::int32_t;
struct SubscriptionsPageState;
struct SubscriptionsDialogState;
struct SubscriptionEditorState;

// Edit one subscription's site, query, filter and timing.
class HydrusSubscriptionEditorDialog : public QDialog {
public:
    HydrusSubscriptionEditorDialog(HydrusMainWindow *main, const SubscriptionEditorState &state, QWidget *parent = nullptr);

    void set_state(const SubscriptionEditorState &state);

private:
    void save();

    HydrusMainWindow *main = nullptr;
    QString original;
    QString page_id;
    QLineEdit *name = nullptr;
    QLineEdit *url = nullptr;
    QLineEdit *query = nullptr;
    QComboBox *site = nullptr;
    QLineEdit *filter = nullptr;
    QSpinBox *period = nullptr;
    QCheckBox *paused = nullptr;
};

// one table of subscriptions and the batch action rows underneath.
class HydrusSubscriptionsDialog : public QDialog {
public:
    HydrusSubscriptionsDialog(HydrusMainWindow *main, const SubscriptionsDialogState &state, QWidget *parent = nullptr);

    void set_state(const SubscriptionsDialogState &state);

private:
    void add_subscription();
    void with_selected(UiAction action);
    void import_options_action(const QString &action);
    void refresh_buttons();
    QStringList selected_names() const;

    HydrusMainWindow *main = nullptr;
    std::vector<std::pair<QString, QString>> sites;
    std::map<QString, QString> import_options;
    QTableWidget *table = nullptr;
    QLabel *warning = nullptr;
    QList<QWidget *> selection_buttons;
};

// Subscription manager page.
class HydrusSubscriptionsPage : public QWidget {
public:
    HydrusSubscriptionsPage(HydrusMainWindow *main, const QString &page_id, const SubscriptionsPageState &state);

    const QString &page_id() const;
    void update_state(const SubscriptionsPageState &state);

private:
    void add_subscription();
    void with_selected(UiAction action);

    HydrusMainWindow *main = nullptr;
    QString id;
    std::vector<std::pair<QString, QString>> sites;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
    QPushButton *import_options_button = nullptr;
};
