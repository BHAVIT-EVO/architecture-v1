import Foundation
import Combine
import SwiftUI

extension HomePageIndicatorSize: StoredSettingValue {}

@MainActor
final class HomePageSettingsStore: SettingsStoreBase {
    @StoredDefault(key: GeneralSettingsStorage.Keys.homePageLiveActivity, defaultValue: true)
    var isHomePageLiveActivityEnabled: Bool

    @StoredDefault(key: GeneralSettingsStorage.Keys.homePagePageIndicator, defaultValue: true)
    var isHomePagePageIndicatorEnabled: Bool

    @StoredDefault(key: GeneralSettingsStorage.Keys.homePageIndicatorSize, defaultValue: .medium)
    var homePageIndicatorSize: HomePageIndicatorSize

    @Published var homePageOrder: [HomePages] {
        didSet {
            persist(homePageOrder.map { $0.rawValue }, for: GeneralSettingsStorage.Keys.homePageOrder)
        }
    }

    @Published var homePageDisabled: Set<HomePages> {
        didSet {
            persist(Array(homePageDisabled).map { $0.rawValue }, for: GeneralSettingsStorage.Keys.homePageDisabled)
            if homePageDisabled.count == HomePages.allCases.count {
                isHomePageLiveActivityEnabled = false
            } else if !isHomePageLiveActivityEnabled && oldValue.count == HomePages.allCases.count {
                isHomePageLiveActivityEnabled = true
            }
        }
    }

    func resetHomePage() {
        isHomePageLiveActivityEnabled = true
        homePageOrder = HomePages.allCases
        homePageDisabled = Set<HomePages>()
        isHomePagePageIndicatorEnabled = true
        homePageIndicatorSize = .medium
    }

    override init(defaults: UserDefaults) {
        let savedOrder = (defaults.array(forKey: GeneralSettingsStorage.Keys.homePageOrder) as? [String]) ??
            ((GeneralSettingsStorage.defaultValues[GeneralSettingsStorage.Keys.homePageOrder] as? [String]) ?? [])
        var parsedOrder = savedOrder.compactMap { HomePages(rawValue: $0) }
        if parsedOrder.isEmpty {
            parsedOrder = HomePages.allCases
        } else {
            for page in HomePages.allCases {
                if !parsedOrder.contains(page) {
                    parsedOrder.append(page)
                }
            }
        }
        self.homePageOrder = parsedOrder

        let savedDisabled = (defaults.array(forKey: GeneralSettingsStorage.Keys.homePageDisabled) as? [String]) ??
            ((GeneralSettingsStorage.defaultValues[GeneralSettingsStorage.Keys.homePageDisabled] as? [String]) ?? [])
        self.homePageDisabled = Set(savedDisabled.compactMap { HomePages(rawValue: $0) })

        super.init(defaults: defaults)
    }
}
