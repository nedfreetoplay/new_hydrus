#pragma once

// WS-15.1.4: subscription widgets moved verbatim from bridge.hpp (move-only split).

#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QString>
#include <QStringList>
#include <QTableWidget>
#include <QWidget>

class HydrusMainWindow;

// Edit one subscription's site, query, filter and timing.
class HydrusSubscriptionEditorDialog : public QDialog {
public:
    HydrusSubscriptionEditorDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void save();

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
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
    HydrusSubscriptionsDialog(HydrusMainWindow *main, const QJsonObject &state, QWidget *parent = nullptr);

    void set_state(const QJsonObject &state);

private:
    void add_subscription();
    void with_selected(const QString &action);
    void import_options_action(const QString &action);
    void refresh_buttons();
    QStringList selected_names() const;

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QTableWidget *table = nullptr;
    QLabel *warning = nullptr;
    QList<QWidget *> selection_buttons;
};

// Subscription manager page.
class HydrusSubscriptionsPage : public QWidget {
public:
    HydrusSubscriptionsPage(HydrusMainWindow *main, const QString &page_id, const QJsonObject &state);

    const QString &page_id() const;
    void update_state(const QJsonObject &state);

private:
    void add_subscription();
    void with_selected(const QString &action);

    HydrusMainWindow *main = nullptr;
    QString id;
    QTableWidget *table = nullptr;
    QLabel *status = nullptr;
    QJsonArray sites;
    QPushButton *import_options_button = nullptr;
};
